import 'package:xayn_discovery_engine/src/api/api.dart'
    show ClientEvent, EngineEvent;
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart';
import 'package:xayn_discovery_engine/src/discovery_engine_manager.dart'
    show DiscoveryEngineManager;
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';

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
  ///     apiBaseUrl: 'https://xaynapi.xayn.dev',
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
        // throw DiscoveryEngineInitException();
      }

      return DiscoveryEngine._(manager);
    } catch (e) {
      //
      throw DiscoveryEngineInitException('something went very wrong');
    }
  }

  /// Resets the AI (start fresh).
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

  /// Changes configuration for the news feed
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

  /// Requests initial news feed.
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
  /// **IMPORTANT!**
  ///
  /// Use when the [Document]s are no longer available to the user.
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
  /// Please use:
  /// - [DocumentFeedback.positive] to indicate that the [Document] was **liked**
  /// - [DocumentFeedback.negative] to indicate that the [Document] was **diliked**
  /// - [DocumentFeedback.neutral] as a default **neutral** state of the [Document].
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

  /// Registers that the user stopped reading a [Document].
  ///
  /// Should be used when the user stops reading a [Document], either by going
  /// back to documents list or by navigating further to another [Document].
  Future<EngineEvent> closeDocument(DocumentId documentId) {
    return _trySend(() async {
      final event = ClientEvent.documentClosed(documentId);
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
      // into `EngineExceptionRaised` event with a specific reason
      return const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.genericError,
      );
    }
  }
}

class DiscoveryEngineInitException implements Exception {
  final String message;

  DiscoveryEngineInitException(this.message);

  @override
  String toString() {
    return 'DiscoveryEngineInitException{ $message }';
  }
}
