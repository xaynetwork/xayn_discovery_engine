// Copyright 2021 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

import 'dart:async' show StreamSubscription;
import 'package:universal_platform/universal_platform.dart'
    show UniversalPlatform;
import 'package:xayn_discovery_engine/src/api/api.dart'
    show
        ClientEvent,
        ClientEventSucceeded,
        AssetsStatusEngineEvent,
        Configuration,
        DocumentFeedback,
        DocumentId,
        DocumentViewMode,
        EngineEvent,
        EngineExceptionReason,
        FeedMarkets,
        FeedRequestFailed,
        FeedRequestSucceeded,
        NextFeedBatchAvailable,
        NextFeedBatchRequestFailed,
        NextFeedBatchRequestSucceeded;
import 'package:xayn_discovery_engine/src/discovery_engine_manager.dart'
    show DiscoveryEngineManager;
import 'package:xayn_discovery_engine/src/discovery_engine_worker.dart'
    as entry_point show main;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/logger.dart' show logger;
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show
        ConverterException,
        EngineInitException,
        ManagerDisposedException,
        ResponseTimeoutException;

/// A constant that is true if the application was compiled to run on the web.
final kIsWeb = UniversalPlatform.isWeb;

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
  ///     assetsUrl: 'https://ai-assets.dev',
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
    void Function(EngineEvent event)? onAssetsProgress,
    Object? entryPoint,
  }) async {
    try {
      entryPoint ??= kIsWeb ? null : entry_point.main;
      final manager = await DiscoveryEngineManager.create(entryPoint);
      StreamSubscription<EngineEvent>? subscription;

      if (onAssetsProgress != null) {
        subscription = manager.responses
            .where((event) => event is AssetsStatusEngineEvent)
            .listen(onAssetsProgress);
      }

      final initEvent = ClientEvent.init(configuration);
      final response = await manager.send(initEvent, timeout: null);
      await subscription?.cancel();

      if (response is! ClientEventSucceeded) {
        await manager.dispose();
        throw EngineInitException(
          'Something went wrong when sending over the configuration',
          response,
        );
      }

      return DiscoveryEngine._(manager);
    } catch (error) {
      // rethrow exception thrown by issue with configuration
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
    FeedMarkets? feedMarkets,
    int? maxItemsPerFeedBatch,
  }) async {
    return _trySend(() async {
      final event = ClientEvent.configurationChanged(
        feedMarkets: feedMarkets,
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

  /// Logs the time in seconds spent by a user on a [Document] in a certain
  /// mode.
  ///
  /// In response it can return:
  /// - [ClientEventSucceeded] indicating a successful operation
  /// - [EngineExceptionReason] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> logDocumentTime({
    required DocumentId documentId,
    required DocumentViewMode mode,
    required int seconds,
  }) {
    return _trySend(() async {
      final event = ClientEvent.documentTimeSpent(documentId, mode, seconds);
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

  /// Send a [ClientEvent] to the [DiscoveryEngine] and wait for a response.
  Future<EngineEvent> send(ClientEvent event) =>
      _trySend(() => _manager.send(event));

  /// Performs a cleanup that includes closing all communication channels
  /// and disposing the underlying PlatformWorker.
  Future<void> dispose() => _manager.dispose();

  Future<EngineEvent> _trySend(Future<EngineEvent> Function() fn) async {
    try {
      // we need to await the result otherwise catch won't work
      return await fn();
    } catch (e) {
      var reason = EngineExceptionReason.genericError;

      if (e is ConverterException) {
        reason = EngineExceptionReason.converterException;
      } else if (e is ResponseTimeoutException) {
        reason = EngineExceptionReason.responseTimeout;
      } else if (e is ManagerDisposedException) {
        reason = EngineExceptionReason.engineDisposed;
      }

      // log the error
      logger.e(e);

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
    bool? fetchingAssetsStarted,
    bool? fetchingAssetsProgressed,
    bool? fetchingAssetsFinished,
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
        fetchingAssetsStarted: _maybePassThrough(fetchingAssetsStarted),
        fetchingAssetsProgressed: _maybePassThrough(fetchingAssetsProgressed),
        fetchingAssetsFinished: _maybePassThrough(fetchingAssetsFinished),
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
