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

  static Future<DiscoveryEngine> init({
    required Configuration configuration,
  }) async {
    try {
      final manager = await DiscoveryEngineManager.create();
      final initEvent = ClientEvent.init(configuration);
      final response = await manager.send(initEvent);

      if (response is! ClientEventSucceeded) {
        throw StateError('something went very wrong');
      }

      return DiscoveryEngine._(manager);
    } catch (e) {
      //
      throw StateError('something went very wrong');
    }
  }

  /// Resets the AI (start fresh).
  Future<EngineEvent> resetEngine() {
    return _trySend(() {
      final event = ClientEvent.resetEngine();
      return _manager.send(event);
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
      return _manager.send(event);
    });
  }

  /// Requests initial news feed.
  Future<EngineEvent> requestFeed() {
    return _trySend(() async {
      final event = ClientEvent.feedRequested();
      return _manager.send(event);
    });
  }

  ///
  Future<EngineEvent> requestNextFeedBatch() {
    return _trySend(() async {
      final event = ClientEvent.nextFeedBatchRequested();
      return _manager.send(event);
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
      return _manager.send(event);
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
      return _manager.send(event);
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
      return _manager.send(event);
    });
  }

  /// Registers that the user stopped reading a [Document].
  ///
  /// Should be used when the user stops reading a [Document], either by going
  /// back to documents list or by navigating further to another [Document].
  Future<EngineEvent> closeDocument(DocumentId documentId) {
    return _trySend(() async {
      final event = ClientEvent.documentClosed(documentId);
      return _manager.send(event);
    });
  }

  Future<EngineEvent> _trySend(Future<EngineEvent> Function() fn) async {
    try {
      // we need to await the result otherwise catch won't work
      return await fn();
    } catch (e) {
      // TODO: introduce mapping of possible exceptions
      // into `EngineExceptionRaised` event with a specific reason
      return EngineExceptionRaised(EngineExceptionReason.genericError);
    }
  }
}
