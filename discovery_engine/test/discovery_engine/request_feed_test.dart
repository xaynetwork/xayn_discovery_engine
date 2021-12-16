import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show
        FeedRequestSucceeded,
        FeedRequestFailed,
        EngineEvent,
        FeedFailureReason,
        EngineExceptionRaised,
        EngineExceptionReason;

import 'utils/utils.dart'
    show
        MockDiscoveryEngineWorker,
        createEngineWithEntryPoint,
        withSuccessResponse;

void main() {
  group('DiscoveryEngine requestFeed', () {
    test(
        'if worker responds with "FeedRequestSucceeded" event it should pass it'
        'as a response of the Discovery Engine', () async {
      final engine = await createEngineWithEntryPoint(withSuccessResponse);

      expect(
        engine.requestFeed(),
        completion(isA<FeedRequestSucceeded>()),
      );
      expect(
        engine.engineEvents,
        emitsInOrder(<EngineEvent>[
          const FeedRequestSucceeded([]),
        ]),
      );
    });

    test(
        'if worker responds with "FeedRequestFailed" event it should pass it'
        'as a response of the Discovery Engine', () async {
      final engine = await createEngineWithEntryPoint(withFailureResponse);
      final response = await engine.requestFeed();

      expect(response, isA<FeedRequestFailed>());
      expect(
        (response as FeedRequestFailed).reason,
        FeedFailureReason.noNewsForMarket,
      );
    });

    test(
        'if worker responds with "EngineExceptionRaised" event it should pass it'
        'as a response of the Discovery Engine', () async {
      final engine = await createEngineWithEntryPoint(withErrorResponse);
      final response = await engine.requestFeed();

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
      final response = await engine.requestFeed();

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
      feedRequestedResponse: const EngineEvent.feedRequestFailed(
        FeedFailureReason.noNewsForMarket,
      ),
    );

void withErrorResponse(Object initialMessage) => MockDiscoveryEngineWorker(
      initialMessage,
      feedRequestedResponse: const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.genericError,
      ),
    );

void withWrongEventResponse(Object initialMessage) => MockDiscoveryEngineWorker(
      initialMessage,
      feedRequestedResponse: const EngineEvent.nextFeedBatchAvailable(),
    );
