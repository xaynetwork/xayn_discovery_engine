import 'package:xayn_discovery_engine/src/api/api.dart'
    show
        ClientEvent,
        ClientEventSucceeded,
        EngineEvent,
        EngineExceptionReason,
        FeedRequestSucceeded,
        FeedRequestFailed,
        NextFeedBatchAvailable,
        NextFeedBatchRequestSucceeded,
        NextFeedBatchRequestFailed;
import 'package:xayn_discovery_engine/src/discovery_engine_manager.dart'
    show DiscoveryEngineManager;
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, DocumentStatus, DocumentFeedback;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// This class exposes Xayn Discovery Engine API to the clients.
class DiscoveryEngine {
  final DiscoveryEngineManager _manager;

  DiscoveryEngine._(this._manager);

  /// Stream of [EngineEvent] coming back from a discovery engine worker.
  Stream<EngineEvent> get engineEvents => _manager.responses;

  /// Initializes the [DiscoveryEngine].
  ///
  /// It can throw [DiscoveryEngineInitException].
  ///
  /// **EXAMPLE**:
  ///
  /// ```
  /// try {
  ///   const config = Configuration(
  ///     apiKey: '**********',
  ///     apiBaseUrl: 'https://example-api.dev',
  ///     feedMarket: 'de-DE',
  ///     maxItemsPerFeedBatch: 50,
  ///     applicationDirectoryPath: './',
  ///   );
  ///
  ///   // Initialize the engine
  ///   final engine = await DiscoveryEngine.init(configuration: config);
  /// } catch (e) {
  ///   // handle exception
  /// }
  /// ```
  static Future<DiscoveryEngine> init({
    // TODO: validation of parameters?
    required Configuration configuration,
  }) async {
    try {
      final manager = await DiscoveryEngineManager.create();
      final initEvent = ClientEvent.init(configuration);
      final response = await manager.send(initEvent);

      // TODO: provide proper error handling during initialization
      if (response is! ClientEventSucceeded) {
        throw DiscoveryEngineInitException('Initialisation failed');
      }

      return DiscoveryEngine._(manager);
    } catch (e) {
      // TODO: provide proper error handling
      throw DiscoveryEngineInitException('something went very wrong');
    }
  }

  /// Resets the AI (fresh start).
  ///
  /// In response it can return:
  /// - [ClientEventSucceeded] indicating a successful operation
  /// - [EngineExceptionReason] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> resetEngine() {
    return _trySend(() async {
      const event = ClientEvent.resetEngine();
      final response = await _manager.send(event);

      return response.maybeMap(
        clientEventSucceeded: (event) => event,
        engineExceptionRaised: (event) => event,
        // in case of a wrong event in response create an EngineExceptionRaised
        orElse: () => const EngineEvent.engineExceptionRaised(
          EngineExceptionReason.wrongEventInResponse,
        ),
      );
    });
  }

  /// Changes configuration for the news feed.
  ///
  /// In response it can return:
  /// - [ClientEventSucceeded] indicating a successful operation
  /// - [EngineExceptionReason] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> changeConfiguration({
    String? feedMarket,
    int? maxItemsPerFeedBatch,
  }) async {
    return _trySend(() async {
      // TODO: validation of parameters?
      final event = ClientEvent.configurationChanged(
        feedMarket: feedMarket,
        maxItemsPerFeedBatch: maxItemsPerFeedBatch,
      );
      final response = await _manager.send(event);

      return response.maybeMap(
        clientEventSucceeded: (event) => event,
        engineExceptionRaised: (event) => event,
        // in case of a wrong event in response create an EngineExceptionRaised
        orElse: () => const EngineEvent.engineExceptionRaised(
          EngineExceptionReason.wrongEventInResponse,
        ),
      );
    });
  }

  /// Requests initial news feed. It should be used as initial request
  /// in the current session.
  ///
  /// In response it can return:
  /// - [FeedRequestSucceeded] for successful response, containing a list of
  /// [Document] items
  /// - [FeedRequestFailed] for failed response, with a reason for failure
  /// - [EngineExceptionReason] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> requestFeed() {
    return _trySend(() async {
      const event = ClientEvent.feedRequested();
      final response = await _manager.send(event);

      return response.maybeMap(
        feedRequestSucceeded: (event) => event,
        feedRequestFailed: (event) => event,
        engineExceptionRaised: (event) => event,
        // in case of a wrong event in response create an EngineExceptionRaised
        orElse: () => const EngineEvent.engineExceptionRaised(
          EngineExceptionReason.wrongEventInResponse,
        ),
      );
    });
  }

  /// Requests next batch of news feed [Document]s.
  ///
  /// Usualy used when reaching the end of the current list of items, in
  /// response to [NextFeedBatchAvailable] event, or after some user action.
  ///
  /// In response it can return:
  /// - [NextFeedBatchRequestSucceeded] for successful response, containing
  /// a list of [Document] items
  /// - [NextFeedBatchRequestFailed] for failed response, with a reason for
  /// failure
  /// - [EngineExceptionReason] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> requestNextFeedBatch() {
    return _trySend(() async {
      const event = ClientEvent.nextFeedBatchRequested();
      final response = await _manager.send(event);

      return response.maybeMap(
        nextFeedBatchRequestSucceeded: (event) => event,
        nextFeedBatchRequestFailed: (event) => event,
        engineExceptionRaised: (event) => event,
        // in case of a wrong event in response create an EngineExceptionRaised
        orElse: () => const EngineEvent.engineExceptionRaised(
          EngineExceptionReason.wrongEventInResponse,
        ),
      );
    });
  }

  /// Closes the [Document]s with specified [DocumentId]s for further modification.
  ///
  /// **IMPORTANT!:**
  /// Use when the [Document]s are no longer available to the user and the user
  /// **can NOT interact** with them.
  ///
  /// In response it can return:
  /// - [ClientEventSucceeded] indicating a successful operation
  /// - [EngineExceptionReason] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> closeFeedDocuments(Set<DocumentId> documentIds) {
    return _trySend(() async {
      final event = ClientEvent.feedDocumentsClosed(documentIds);
      final response = await _manager.send(event);

      return response.maybeMap(
        clientEventSucceeded: (event) => event,
        engineExceptionRaised: (event) => event,
        // in case of a wrong event in response create an EngineExceptionRaised
        orElse: () => const EngineEvent.engineExceptionRaised(
          EngineExceptionReason.wrongEventInResponse,
        ),
      );
    });
  }

  /// Changes the status of a [Document].
  ///
  /// - when the [Document] was presented to the user the status should change
  /// from [DocumentStatus.missed] to [DocumentStatus.presented].
  ///
  /// - when the [Document] was presented but then was scrolled out of the
  /// screen the status should change from [DocumentStatus.presented] to
  /// [DocumentStatus.skipped]. It means the user saw the [Document],
  /// but it wasn't relevant.
  ///
  /// - when the [Document] was opened the status should change from
  /// [DocumentStatus.presented] or [DocumentStatus.skipped] to
  /// [DocumentStatus.opened]. It means the user was interested enough in
  /// the [Document] to open it.
  ///
  /// In response it can return:
  /// - [ClientEventSucceeded] indicating a successful operation
  /// - [EngineExceptionReason] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> changeDocumentStatus({
    required DocumentId documentId,
    required DocumentStatus status,
  }) {
    return _trySend(() async {
      final event = ClientEvent.documentStatusChanged(documentId, status);
      final response = await _manager.send(event);

      return response.maybeMap(
        clientEventSucceeded: (event) => event,
        engineExceptionRaised: (event) => event,
        // in case of a wrong event in response create an EngineExceptionRaised
        orElse: () => const EngineEvent.engineExceptionRaised(
          EngineExceptionReason.wrongEventInResponse,
        ),
      );
    });
  }

  /// Changes the feedback of a [Document].
  ///
  /// [DocumentFeedback] variants are defined as:
  /// - [DocumentFeedback.positive] indicates that the [Document] was **liked**
  /// - [DocumentFeedback.negative] indicates that the [Document] was **diliked**
  /// - [DocumentFeedback.neutral] as a default **neutral** state of the [Document].
  ///
  /// In response it can return:
  /// - [ClientEventSucceeded] indicating a successful operation
  /// - [EngineExceptionReason] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> changeDocumentFeedback({
    required DocumentId documentId,
    required DocumentFeedback feedback,
  }) {
    return _trySend(() async {
      final event = ClientEvent.documentFeedbackChanged(documentId, feedback);
      final response = await _manager.send(event);

      return response.maybeMap(
        clientEventSucceeded: (event) => event,
        engineExceptionRaised: (event) => event,
        // in case of a wrong event in response create an EngineExceptionRaised
        orElse: () => const EngineEvent.engineExceptionRaised(
          EngineExceptionReason.wrongEventInResponse,
        ),
      );
    });
  }

  Future<EngineEvent> _trySend(Future<EngineEvent> Function() fn) async {
    try {
      // we need to await the result otherwise catch won't work
      return await fn();
    } catch (e) {
      // TODO: introduce mapping of possible exceptions
      // into [EngineExceptionRaised] event with a specific reason
      return const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.genericError,
      );
    }
  }
}

// NOTE: this is temporary and it will change
class DiscoveryEngineInitException implements Exception {
  final String message;

  DiscoveryEngineInitException(this.message);

  @override
  String toString() {
    return 'DiscoveryEngineInitException{ $message }';
  }
}
