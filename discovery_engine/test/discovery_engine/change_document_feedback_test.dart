import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show
        ClientEventSucceeded,
        DocumentId,
        EngineEvent,
        EngineExceptionRaised,
        EngineExceptionReason,
        DocumentFeedback;

import 'utils/utils.dart'
    show MockDiscoveryEngineWorker, createEngineWithEntryPoint;

void main() {
  group('DiscoveryEngine changeDocumentFeedback', () {
    test(
        'if worker responds with "ClientEventSucceeded" event it should pass it'
        'as a response of the Discovery Engine', () async {
      final engine = await createEngineWithEntryPoint(withSuccessResponse);

      expect(
        engine.changeDocumentFeedback(
          documentId: DocumentId(),
          feedback: DocumentFeedback.positive,
        ),
        completion(isA<ClientEventSucceeded>()),
      );
    });

    test(
        'if worker responds with "EngineExceptionRaised" event it should pass it'
        'as a response of the Discovery Engine', () async {
      final engine = await createEngineWithEntryPoint(withErrorResponse);
      final response = await engine.changeDocumentFeedback(
        documentId: DocumentId(),
        feedback: DocumentFeedback.positive,
      );

      expect(response, isA<EngineExceptionRaised>());
      expect(
        (response as EngineExceptionRaised).reason,
        EngineExceptionReason.genericError,
      );
    });

    test(
        'if worker responds with something else than allowed events it should '
        'catch it and respond with "EngineExceptionRaised" event '
        'with "wrongEventInResponse" reason', () async {
      final engine = await createEngineWithEntryPoint(withWrongEventResponse);
      final response = await engine.changeDocumentFeedback(
        documentId: DocumentId(),
        feedback: DocumentFeedback.positive,
      );

      expect(response, isA<EngineExceptionRaised>());
      expect(
        (response as EngineExceptionRaised).reason,
        EngineExceptionReason.wrongEventInResponse,
      );
    });
  });
}

void withSuccessResponse(Object initialMessage) =>
    MockDiscoveryEngineWorker(initialMessage);

void withErrorResponse(Object initialMessage) => MockDiscoveryEngineWorker(
      initialMessage,
      documentFeedbackChangedResponse: const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.genericError,
      ),
    );

void withWrongEventResponse(Object initialMessage) => MockDiscoveryEngineWorker(
      initialMessage,
      documentFeedbackChangedResponse:
          const EngineEvent.nextFeedBatchAvailable(),
    );
