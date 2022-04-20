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
        AssetsStatusEngineEvent,
        ClientEvent,
        ClientEventSucceeded,
        Configuration,
        DocumentId,
        DocumentViewMode,
        EngineEvent,
        EngineExceptionReason,
        FeedMarkets,
        NextFeedBatchAvailable,
        NextFeedBatchRequestFailed,
        NextFeedBatchRequestSucceeded,
        ExcludedSourcesListRequestSucceeded,
        ExcludedSourcesListRequestFailed,
        SearchRequestSucceeded,
        SearchRequestFailed,
        NextSearchBatchRequestSucceeded,
        NextSearchBatchRequestFailed,
        SearchTermRequestSucceeded,
        SearchTermRequestFailed,
        RestoreSearchSucceeded,
        RestoreSearchFailed,
        RestoreFeedFailed,
        RestoreFeedSucceeded,
        UserReaction;
import 'package:xayn_discovery_engine/src/discovery_engine_manager.dart'
    show DiscoveryEngineManager;
import 'package:xayn_discovery_engine/src/discovery_engine_worker.dart'
    as entry_point show main;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show Source;
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
  ///     maxItemsPerFeedBatch: 20,
  ///     maxItemsPerSearchBatch: 20,
  ///     feedMarket: {const FeedMarket(countryCode: 'DE', langCode: 'de')},
  ///     applicationDirectoryPath: './',
  ///     manifest: Manifest.fromJson({}),
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
    String? aiConfig,
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

      final initEvent = ClientEvent.init(configuration, aiConfig: aiConfig);
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
    } catch (error, stackTrace) {
      const message =
          'Something went wrong during Discovery Engine initialization';
      logger.e(message, error, stackTrace);
      // rethrow exception thrown by issue with configuration
      if (error is EngineInitException) rethrow;
      // throw for the client to catch
      throw EngineInitException(message, error);
    }
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
    int? maxItemsPerSearchBatch,
  }) async {
    return _trySend(() async {
      final event = ClientEvent.configurationChanged(
        feedMarkets: feedMarkets,
        maxItemsPerFeedBatch: maxItemsPerFeedBatch,
        maxItemsPerSearchBatch: maxItemsPerSearchBatch,
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
  /// - [RestoreFeedSucceeded] for successful response, containing a list of
  /// [Document]s
  /// - [RestoreFeedFailed] for failed response, with a reason for failure
  /// - [EngineExceptionReason] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> restoreFeed() async {
    return _trySend(() async {
      const event = ClientEvent.restoreFeedRequested();
      final response = await _manager.send(event);

      return response.mapEvent(
        restoreFeedSucceeded: true,
        restoreFeedFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Requests next batch of news feed [Document]s.
  ///
  /// Usually used when reaching the end of the current list of items, in
  /// response to [NextFeedBatchAvailable] event, or after some user action.
  ///
  /// In response it can return:
  /// - [NextFeedBatchRequestSucceeded] for successful response, containing
  /// a list of [Document]s
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

  /// Adds a source [Uri] to the set of excluded sources.
  ///
  /// In response it can return:
  /// - [ClientEventSucceeded] indicating a successful operation
  /// - [EngineExceptionReason] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> addSourceToExcludedList(Source source) {
    return _trySend(() async {
      final event = ClientEvent.excludedSourceAdded(source);
      final response = await _manager.send(event);

      return response.mapEvent(
        clientEventSucceeded: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Removes a source [Uri] from the set of excluded sources.
  ///
  /// In response it can return:
  /// - [ClientEventSucceeded] indicating a successful operation
  /// - [EngineExceptionReason] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> removeSourceFromExcludedList(Source source) {
    return _trySend(() async {
      final event = ClientEvent.excludedSourceRemoved(source);
      final response = await _manager.send(event);

      return response.mapEvent(
        clientEventSucceeded: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Returns a [Set<Source>] with excluded sources.
  ///
  /// In response it can return:
  /// - [ExcludedSourcesListRequestSucceeded] indicating a successful operation,
  /// containing a set of sources.
  /// - [ExcludedSourcesListRequestFailed] indicating a failed operation
  /// - [EngineExceptionReason] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> getExcludedSourcesList() {
    return _trySend(() async {
      const event = ClientEvent.excludedSourcesListRequested();
      final response = await _manager.send(event);

      return response.mapEvent(
        excludedSourcesListRequestSucceeded: true,
        excludedSourcesListRequestFailed: true,
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

  /// Changes the user reaction to a [Document].
  ///
  /// [UserReaction] variants are defined as:
  /// - [UserReaction.positive] indicates that the [Document] was **liked**
  /// - [UserReaction.negative] indicates that the [Document] was **diliked**
  /// - [UserReaction.neutral] as a default **neutral** state of the [Document].
  ///
  /// In response it can return:
  /// - [ClientEventSucceeded] indicating a successful operation
  /// - [EngineExceptionReason] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> changeUserReaction({
    required DocumentId documentId,
    required UserReaction userReaction,
  }) {
    return _trySend(() async {
      final event = ClientEvent.userReactionChanged(documentId, userReaction);
      final response = await _manager.send(event);

      return response.mapEvent(
        clientEventSucceeded: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Requests a new search for [Document]s related to `queryTerm`.
  ///
  /// In response it can return:
  /// - [SearchRequestSucceeded] for successful response, containing a list of
  /// [Document]s
  /// - [SearchRequestFailed] for failed response, with a reason for failure
  /// - [EngineExceptionReason] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> requestSearch(String queryTerm) {
    return _trySend(() async {
      final event = ClientEvent.searchRequested(queryTerm, false);
      final response = await _manager.send(event);

      return response.mapEvent(
        searchRequestSucceeded: true,
        searchRequestFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Requests next batch of [Document]s related to the current active search.
  ///
  /// In response it can return:
  /// - [NextSearchBatchRequestSucceeded] for successful response, containing a list of
  /// [Document]s
  /// - [NextSearchBatchRequestFailed] for failed response, with a reason for failure
  /// - [EngineExceptionReason] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> requestNextSearchBatch() {
    return _trySend(() async {
      const event = ClientEvent.nextSearchBatchRequested();
      final response = await _manager.send(event);

      return response.mapEvent(
        nextSearchBatchRequestSucceeded: true,
        nextSearchBatchRequestFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Restores latest active search that wasn't closed.
  ///
  /// In response it can return:
  /// - [RestoreSearchSucceeded] for successful response, containing a list of
  /// [Document]s
  /// - [RestoreSearchFailed] for failed response, with a reason for failure
  /// - [EngineExceptionReason] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> restoreSearch() {
    return _trySend(() async {
      const event = ClientEvent.restoreSearchRequested();
      final response = await _manager.send(event);

      return response.mapEvent(
        restoreSearchSucceeded: true,
        restoreSearchFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Returns the current search term.
  ///
  /// In response it can return:
  /// - [SearchTermRequestSucceeded] for successful response, containing the
  /// search term
  /// - [SearchTermRequestFailed] for failed response, with a reason for failure
  /// - [EngineExceptionReason] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> getSearchTerm() {
    return _trySend(() async {
      const event = ClientEvent.searchTermRequested();
      final response = await _manager.send(event);

      return response.mapEvent(
        searchTermRequestSucceeded: true,
        searchTermRequestFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Closes the [Document]s related to current active search for further
  /// modification.
  ///
  /// **IMPORTANT!:**
  /// Use when the [Document]s are no longer available to the user and the user
  /// **can NOT interact** with them.
  ///
  /// In response it can return:
  /// - [ClientEventSucceeded] indicating a successful operation
  /// - [EngineExceptionReason] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> closeSearch() {
    return _trySend(() async {
      const event = ClientEvent.searchClosed();
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
    } catch (error, stackTrace) {
      var reason = EngineExceptionReason.genericError;

      if (error is ConverterException) {
        reason = EngineExceptionReason.converterException;
      } else if (error is ResponseTimeoutException) {
        reason = EngineExceptionReason.responseTimeout;
      } else if (error is ManagerDisposedException) {
        reason = EngineExceptionReason.engineDisposed;
      }

      const message = 'Something went wrong when trying to send a ClientEvent';
      // log the error
      logger.e(message, error, stackTrace);

      // into [EngineExceptionRaised] event with a specific reason
      return EngineEvent.engineExceptionRaised(
        reason,
        message: '$error',
        stackTrace: '$stackTrace',
      );
    }
  }
}

extension _MapEvent on EngineEvent {
  EngineEvent mapEvent({
    bool? restoreFeedSucceeded,
    bool? restoreFeedFailed,
    bool? nextFeedBatchRequestSucceeded,
    bool? nextFeedBatchRequestFailed,
    bool? nextFeedBatchAvailable,
    bool? excludedSourcesListRequestSucceeded,
    bool? excludedSourcesListRequestFailed,
    bool? fetchingAssetsStarted,
    bool? fetchingAssetsProgressed,
    bool? fetchingAssetsFinished,
    bool? clientEventSucceeded,
    bool? engineExceptionRaised,
    bool? documentsUpdated,
    bool? searchRequestSucceeded,
    bool? searchRequestFailed,
    bool? nextSearchBatchRequestSucceeded,
    bool? nextSearchBatchRequestFailed,
    bool? restoreSearchSucceeded,
    bool? restoreSearchFailed,
    bool? searchTermRequestSucceeded,
    bool? searchTermRequestFailed,
    bool? trustedSourcesListRequestSucceeded,
    bool? trustedSourcesListRequestFailed,
  }) =>
      map(
        restoreFeedSucceeded: _maybePassThrough(restoreFeedSucceeded),
        restoreFeedFailed: _maybePassThrough(restoreFeedFailed),
        nextFeedBatchRequestSucceeded:
            _maybePassThrough(nextFeedBatchRequestSucceeded),
        nextFeedBatchRequestFailed:
            _maybePassThrough(nextFeedBatchRequestFailed),
        nextFeedBatchAvailable: _maybePassThrough(nextFeedBatchAvailable),
        excludedSourcesListRequestSucceeded:
            _maybePassThrough(excludedSourcesListRequestSucceeded),
        excludedSourcesListRequestFailed:
            _maybePassThrough(excludedSourcesListRequestFailed),
        fetchingAssetsStarted: _maybePassThrough(fetchingAssetsStarted),
        fetchingAssetsProgressed: _maybePassThrough(fetchingAssetsProgressed),
        fetchingAssetsFinished: _maybePassThrough(fetchingAssetsFinished),
        clientEventSucceeded: _maybePassThrough(clientEventSucceeded),
        engineExceptionRaised: _maybePassThrough(engineExceptionRaised),
        documentsUpdated: _maybePassThrough(documentsUpdated),
        searchRequestSucceeded: _maybePassThrough(searchRequestSucceeded),
        searchRequestFailed: _maybePassThrough(searchRequestFailed),
        nextSearchBatchRequestSucceeded:
            _maybePassThrough(nextSearchBatchRequestSucceeded),
        nextSearchBatchRequestFailed:
            _maybePassThrough(nextSearchBatchRequestFailed),
        restoreSearchSucceeded: _maybePassThrough(restoreSearchSucceeded),
        restoreSearchFailed: _maybePassThrough(restoreSearchFailed),
        searchTermRequestSucceeded:
            _maybePassThrough(searchTermRequestSucceeded),
        searchTermRequestFailed: _maybePassThrough(searchTermRequestFailed),
        trustedSourcesListRequestSucceeded:
            _maybePassThrough(trustedSourcesListRequestSucceeded),
        trustedSourcesListRequestFailed:
            _maybePassThrough(trustedSourcesListRequestFailed),
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
