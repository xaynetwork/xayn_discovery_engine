// Copyright 2022 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

import 'dart:async' show unawaited;

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show
        ClientEvent,
        EngineEvent,
        EngineExceptionReason,
        NextFeedBatchRequestSucceeded,
        RestoreFeedSucceeded;
import 'package:xayn_discovery_engine/src/discovery_engine_worker.dart'
    show DiscoveryEngineWorker;
import 'package:xayn_discovery_engine/src/worker/common/oneshot.dart'
    show OneshotRequest;

import '../logging.dart' show setupLogging;
import 'utils/utils.dart' show createEngineWithEntryPoint;

void main() {
  setupLogging();

  group('DiscoveryEngine event concurrency', () {
    test('incoming events should be processed sequentially', () async {
      final engine = await createEngineWithEntryPoint(withCustomEventHandler);

      unawaited(engine.restoreFeed());
      unawaited(engine.requestNextFeedBatch());
      unawaited(engine.restoreFeed());
      unawaited(engine.requestNextFeedBatch());
      unawaited(engine.requestNextFeedBatch());
      unawaited(engine.restoreFeed());

      expect(
        engine.engineEvents,
        emitsInOrder(<EngineEvent>[
          const RestoreFeedSucceeded([]),
          const NextFeedBatchRequestSucceeded([]),
          const RestoreFeedSucceeded([]),
          const NextFeedBatchRequestSucceeded([]),
          const NextFeedBatchRequestSucceeded([]),
          const RestoreFeedSucceeded([]),
        ]),
      );
    });
  });
}

void withCustomEventHandler(Object initialMessage) =>
    MockedDiscoveryEngineWorker(initialMessage);

class MockedDiscoveryEngineWorker extends DiscoveryEngineWorker {
  MockedDiscoveryEngineWorker(Object message) : super(message);

  @override
  Future<void> handleMessage(OneshotRequest<ClientEvent> request) async {
    final response = await request.payload.maybeWhen(
      init: (configuration, deConfig) async =>
          const EngineEvent.engineInitSucceeded(null),
      restoreFeedRequested: () async {
        await Future<void>.delayed(const Duration(milliseconds: 300));
        return const EngineEvent.restoreFeedSucceeded([]);
      },
      nextFeedBatchRequested: () async {
        await Future<void>.delayed(const Duration(milliseconds: 100));
        return const EngineEvent.nextFeedBatchRequestSucceeded([]);
      },
      orElse: () async => const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.genericError,
      ),
    );

    send(response, request.sender);
  }
}
