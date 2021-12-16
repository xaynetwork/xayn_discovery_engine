import 'dart:isolate' show SendPort;

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

    test('when passing a bad entrypoint it should throw "EngineInitException"',
        () {
      void wrongTypeSignature() {}
      void nonStaticOrTopLevelFunction(SendPort sendPort) {}

      expect(
        createEngineWithEntryPoint(wrongTypeSignature),
        throwsA(isA<EngineInitException>()),
      );
      expect(
        createEngineWithEntryPoint(nonStaticOrTopLevelFunction),
        throwsA(isA<EngineInitException>()),
      );
    });

    test(
        'if the response to the "Init" event is different then '
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
