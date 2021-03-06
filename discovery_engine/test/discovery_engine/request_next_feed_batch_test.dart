// Copyright 2021 Xayn AG
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

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show
        NextFeedBatchRequestSucceeded,
        NextFeedBatchRequestFailed,
        EngineEvent,
        FeedFailureReason,
        EngineExceptionRaised,
        EngineExceptionReason;

import '../logging.dart' show setupLogging;
import 'utils/utils.dart'
    show
        MockDiscoveryEngineWorker,
        createEngineWithEntryPoint,
        withSuccessResponse;

void main() {
  setupLogging();

  group('DiscoveryEngine requestNextFeedBatch', () {
    test(
        'if worker responds with "NextFeedBatchRequestSucceeded" event it should pass it'
        'as a response of the Discovery Engine', () async {
      final engine = await createEngineWithEntryPoint(withSuccessResponse);

      expect(
        engine.requestNextFeedBatch(),
        completion(isA<NextFeedBatchRequestSucceeded>()),
      );
      expect(
        engine.engineEvents,
        emitsInOrder(<EngineEvent>[
          const NextFeedBatchRequestSucceeded([]),
        ]),
      );
    });

    test(
        'if worker responds with "NextFeedBatchRequestFailed" event it should pass it'
        'as a response of the Discovery Engine', () async {
      final engine = await createEngineWithEntryPoint(withFailureResponse);
      final response = await engine.requestNextFeedBatch();

      expect(response, isA<NextFeedBatchRequestFailed>());
      expect(
        (response as NextFeedBatchRequestFailed).reason,
        FeedFailureReason.noNewsForMarket,
      );
    });

    test(
        'if worker responds with "EngineExceptionRaised" event it should pass it'
        'as a response of the Discovery Engine', () async {
      final engine = await createEngineWithEntryPoint(withErrorResponse);
      final response = await engine.requestNextFeedBatch();

      expect(response, isA<EngineExceptionRaised>());
      expect(
        (response as EngineExceptionRaised).reason,
        EngineExceptionReason.genericError,
      );
    });

    test(
        'if worker responds with something other than allowed events it should '
        'catch it and respond with "EngineExceptionRaised" event '
        'with "wrongEventInResponse" reason', () async {
      final engine = await createEngineWithEntryPoint(withWrongEventResponse);
      final response = await engine.requestNextFeedBatch();

      expect(response, isA<EngineExceptionRaised>());
      expect(
        (response as EngineExceptionRaised).reason,
        EngineExceptionReason.wrongEventInResponse,
      );
    });
  });
}

void withFailureResponse(Object initialMessage) => MockDiscoveryEngineWorker(
      initialMessage,
      nextFeedBatchRequestedResponse:
          const EngineEvent.nextFeedBatchRequestFailed(
        FeedFailureReason.noNewsForMarket,
      ),
    );

void withErrorResponse(Object initialMessage) => MockDiscoveryEngineWorker(
      initialMessage,
      nextFeedBatchRequestedResponse: const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.genericError,
      ),
    );

void withWrongEventResponse(Object initialMessage) => MockDiscoveryEngineWorker(
      initialMessage,
      nextFeedBatchRequestedResponse: const EngineEvent.clientEventSucceeded(),
    );
