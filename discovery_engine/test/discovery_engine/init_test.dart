import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show
        DiscoveryEngine,
        EngineEvent,
        EngineExceptionReason,
        EngineInitException;

import 'utils/utils.dart'
    show
        MockDiscoveryEngineWorker,
        createEngineWithEntryPoint,
        withSuccessResponse;

void main() {
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
