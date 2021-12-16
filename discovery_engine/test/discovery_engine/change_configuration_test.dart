import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show
        EngineEvent,
        ClientEventSucceeded,
        EngineExceptionRaised,
        EngineExceptionReason;

import 'utils/utils.dart'
    show
        MockDiscoveryEngineWorker,
        createEngineWithEntryPoint,
        withSuccessResponse;

void main() {
  group('DiscoveryEngine changeConfiguration', () {
    test(
        'if worker responds with "ClientEventSucceeded" event it should pass it'
        'as a response of the Discovery Engine', () async {
      final engine = await createEngineWithEntryPoint(withSuccessResponse);

      expect(
        engine.changeConfiguration(maxItemsPerFeedBatch: 10),
        completion(isA<ClientEventSucceeded>()),
      );
      expect(
        engine.engineEvents,
        emitsInOrder(<EngineEvent>[
          const ClientEventSucceeded(),
        ]),
      );
    });

    test(
        'if worker responds with "EngineExceptionRaised" event it should pass it'
        'as a response of the Discovery Engine', () async {
      final engine = await createEngineWithEntryPoint(withErrorResponse);
      final response = await engine.changeConfiguration(feedMarket: 'de-DE');

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
      final response =
          await engine.changeConfiguration(maxItemsPerFeedBatch: 0);

      expect(response, isA<EngineExceptionRaised>());
      expect(
        (response as EngineExceptionRaised).reason,
        EngineExceptionReason.wrongEventInResponse,
      );
    });
  });
}

void withErrorResponse(Object initialMessage) => MockDiscoveryEngineWorker(
      initialMessage,
      configurationChangedResponse: const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.genericError,
      ),
    );

void withWrongEventResponse(Object initialMessage) => MockDiscoveryEngineWorker(
      initialMessage,
      configurationChangedResponse: const EngineEvent.nextFeedBatchAvailable(),
    );
