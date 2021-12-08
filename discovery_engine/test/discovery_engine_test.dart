import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart';

void main() {
  const config = Configuration(
    apiKey: '**********',
    apiBaseUrl: 'https://example-api.dev',
    feedMarket: 'de-DE',
    maxItemsPerFeedBatch: 50,
    applicationDirectoryPath: './',
  );

  group('DiscoveryEngine initialization', () {
    test(
        'when calling "init" it will create and initialize a "DiscoveryEngine" instance',
        () async {
      final engineFuture = DiscoveryEngine.init(configuration: config);
      expect(engineFuture, completion(isA<DiscoveryEngine>()));
    });
  });

  group('DiscoveryEngine basic smoke tests', () {
    late DiscoveryEngine engine;

    setUp(() async {
      engine = await DiscoveryEngine.init(configuration: config);
    });

    test(
        'when sending a "ResetEngine" event it responds with "ClientEventSucceeded"',
        () {
      final respFuture = engine.resetEngine();

      expect(respFuture, completion(isA<ClientEventSucceeded>()));
      expect(
        engine.engineEvents,
        emitsInOrder(<EngineEvent>[
          const ClientEventSucceeded(),
        ]),
      );
    });

    test(
        'when sending a "ConfigurationChanged" event it responds with "ClientEventSucceeded"',
        () {
      final respFuture = engine.changeConfiguration(maxItemsPerFeedBatch: 10);
      expect(respFuture, completion(isA<ClientEventSucceeded>()));
      expect(
        engine.engineEvents,
        emitsInOrder(<EngineEvent>[
          const ClientEventSucceeded(),
        ]),
      );
    });

    test(
        'when sending a "FeedRequested" event it responds with "FeedRequestSucceeded"',
        () async {
      final respFuture = engine.requestFeed();
      expect(respFuture, completion(isA<FeedRequestSucceeded>()));
      expect(
        engine.engineEvents,
        emitsInOrder(<EngineEvent>[
          const FeedRequestSucceeded([]),
        ]),
      );
    });

    test(
        'when sending a "NextFeedBatchRequested" event it responds with "NextFeedBatchRequestSucceeded"',
        () async {
      final respFuture = engine.requestNextFeedBatch();
      expect(respFuture, completion(isA<NextFeedBatchRequestSucceeded>()));
      expect(
        engine.engineEvents,
        emitsInOrder(<EngineEvent>[
          const NextFeedBatchRequestSucceeded([]),
        ]),
      );
    });

    test(
        'when sending a "DocumentFeedbackChanged" event it responds with "ClientEventSucceeded"',
        () async {
      final respFuture = engine.changeDocumentFeedback(
        documentId: DocumentId(),
        feedback: DocumentFeedback.positive,
      );
      expect(respFuture, completion(isA<ClientEventSucceeded>()));
      expect(
        engine.engineEvents,
        emitsInOrder(<EngineEvent>[
          const ClientEventSucceeded(),
        ]),
      );
    });

    test(
        'when sending a "FeedDocumentsClosed" event it responds with "ClientEventSucceeded"',
        () async {
      final respFuture = engine.closeFeedDocuments({DocumentId()});
      expect(respFuture, completion(isA<ClientEventSucceeded>()));
      expect(
        engine.engineEvents,
        emitsInOrder(<EngineEvent>[
          const ClientEventSucceeded(),
        ]),
      );
    });
  });
}
