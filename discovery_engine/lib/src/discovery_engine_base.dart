import 'package:xayn_discovery_engine/src/api/api.dart'
    show
        ClientEvent,
        ClientEventSucceeded,
        EngineEvent,
        EngineExceptionReason,
        FeedRequestFailed,
        FeedRequestSucceeded,
        NextFeedBatchAvailable,
        NextFeedBatchRequestFailed,
        NextFeedBatchRequestSucceeded;
import 'package:xayn_discovery_engine/src/discovery_engine_manager.dart'
    show DiscoveryEngineManager;
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, DocumentStatus, DocumentFeedback;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show
        EngineInitException,
        ResponseEmptyException,
        ResponseTimeoutException,
        ConverterException;

/// This class exposes Xayn Discovery Engine API to the clients.
class DiscoveryEngine {
  final DiscoveryEngineManager _manager;

  DiscoveryEngine._(this._manager);

  /// Stream of [EngineEvent] coming back from a discovery engine worker.
  Stream<EngineEvent> get engineEvents => _manager.responses;

  /// Initializes the [DiscoveryEngine].
  ///
  /// It can throw [EngineInitException].
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
    required Configuration configuration,
  }) async {
    try {
      final manager = await DiscoveryEngineManager.create();
      final initEvent = ClientEvent.init(configuration);
      final response = await manager.send(initEvent);

      if (response is! ClientEventSucceeded) {
        await manager.dispose();
        throw EngineInitException(
          'Something went wrong when sending over the configuration',
          response,
        );
      }

      return DiscoveryEngine._(manager);
    } catch (error) {
      if (error is EngineInitException) rethrow;
      // throw for the client to catch
      throw EngineInitException(
        'Something went wrong during Discovery Engine initialization',
        error,
      );
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

      return response.mapEvent(
        clientEventSucceeded: true,
        engineExceptionRaised: true,
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
      final event = ClientEvent.configurationChanged(
        feedMarket: feedMarket,
        maxItemsPerFeedBatch: maxItemsPerFeedBatch,
      );
      final response = await _manager.send(event);

      return response.mapEvent(
        clientEventSucceeded: true,
        engineExceptionRaised: true,
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
  Future<EngineEvent> requestFeed() async {
    return _trySend(() async {
      const event = ClientEvent.feedRequested();
      final response = await _manager.send(event);

      return response.mapEvent(
        feedRequestSucceeded: true,
        feedRequestFailed: true,
        engineExceptionRaised: true,
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

      return response.mapEvent(
        nextFeedBatchRequestSucceeded: true,
        nextFeedBatchRequestFailed: true,
        engineExceptionRaised: true,
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

      return response.mapEvent(
        clientEventSucceeded: true,
        engineExceptionRaised: true,
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

      return response.mapEvent(
        clientEventSucceeded: true,
        engineExceptionRaised: true,
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

      return response.mapEvent(
        clientEventSucceeded: true,
        engineExceptionRaised: true,
      );
    });
  }

  Future<EngineEvent> _trySend(Future<EngineEvent> Function() fn) async {
    try {
      // we need to await the result otherwise catch won't work
      return await fn();
    } catch (e) {
      // TODO: add proper logging
      print(e);
      var reason = EngineExceptionReason.genericError;

      if (e is ConverterException) {
        reason = EngineExceptionReason.converterException;
      } else if (e is ResponseEmptyException) {
        reason = EngineExceptionReason.emptyResponse;
      } else if (e is ResponseTimeoutException) {
        reason = EngineExceptionReason.responseTimeout;
      }

      // into [EngineExceptionRaised] event with a specific reason
      return EngineEvent.engineExceptionRaised(reason);
    }
  }
}

extension _MapEvent on EngineEvent {
  EngineEvent mapEvent({
    bool? feedRequestSucceeded,
    bool? feedRequestFailed,
    bool? nextFeedBatchRequestSucceeded,
    bool? nextFeedBatchRequestFailed,
    bool? nextFeedBatchAvailable,
    bool? clientEventSucceeded,
    bool? engineExceptionRaised,
  }) =>
      map(
        feedRequestSucceeded: _maybePassThrough(feedRequestSucceeded),
        feedRequestFailed: _maybePassThrough(feedRequestFailed),
        nextFeedBatchRequestSucceeded:
            _maybePassThrough(nextFeedBatchRequestSucceeded),
        nextFeedBatchRequestFailed:
            _maybePassThrough(nextFeedBatchRequestFailed),
        nextFeedBatchAvailable: _maybePassThrough(nextFeedBatchAvailable),
        clientEventSucceeded: _maybePassThrough(clientEventSucceeded),
        engineExceptionRaised: _maybePassThrough(engineExceptionRaised),
      );

  EngineEvent Function(EngineEvent) _maybePassThrough(bool? condition) {
    return condition ?? false ? _passThrough : _orElse;
  }

  // just pass through the original event
  EngineEvent _passThrough(EngineEvent event) => event;

  // in case of a wrong event in response create an EngineExceptionRaised
  EngineEvent _orElse(EngineEvent _event) =>
      const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.wrongEventInResponse,
      );
}
