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
        DiscoveryEngine,
        EngineEvent,
        EngineExceptionReason,
        EngineInitException;

import '../logging.dart' show setupLogging;
import 'utils/utils.dart'
    show
        MockDiscoveryEngineWorker,
        createEngineWithEntryPoint,
        withSuccessResponse;

void main() {
  setupLogging();

  group('DiscoveryEngine init', () {
    test(
        'when calling "init" it should create and initialize '
        'a "DiscoveryEngine" instance', () async {
      final engine = await createEngineWithEntryPoint(withSuccessResponse);

      expect(engine, isA<DiscoveryEngine>());
    });

    test('when passing a bad entry point it should throw "EngineInitException"',
        () {
      void wrongTypeSignature() {}

      expect(
        createEngineWithEntryPoint(wrongTypeSignature),
        throwsA(isA<EngineInitException>()),
      );
    });

    test(
        'if the response to the "Init" event is different to '
        '"ClientEventSucceeded" it should throw "EngineInitException"', () {
      expect(
        createEngineWithEntryPoint(withWrongEventResponse),
        throwsA(isA<EngineInitException>()),
      );
    });
  });
}

void withWrongEventResponse(Object initialMessage) => MockDiscoveryEngineWorker(
      initialMessage,
      initResponse: const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.genericError,
      ),
    );
