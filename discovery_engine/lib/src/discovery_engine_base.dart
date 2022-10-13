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
        AvailableSourcesListRequestSucceeded,
        AvailableSourcesListRequestFailed,
        ClientEvent,
        ClientEventSucceeded,
        Configuration,
        DocumentId,
        DocumentViewMode,
        EngineEvent,
        EngineExceptionRaised,
        EngineExceptionReason,
        FeedMarkets,
        NextFeedBatchAvailable,
        NextFeedBatchRequestFailed,
        NextFeedBatchRequestSucceeded,
        ExcludedSourcesListRequestSucceeded,
        ExcludedSourcesListRequestFailed,
        SetSourcesRequestSucceeded,
        SetSourcesRequestFailed,
        ActiveSearchRequestSucceeded,
        ActiveSearchRequestFailed,
        NextActiveSearchBatchRequestSucceeded,
        NextActiveSearchBatchRequestFailed,
        ActiveSearchTermRequestSucceeded,
        ActiveSearchTermRequestFailed,
        ActiveSearchClosedSucceeded,
        ActiveSearchClosedFailed,
        RestoreActiveSearchSucceeded,
        RestoreActiveSearchFailed,
        DeepSearchRequestSucceeded,
        DeepSearchRequestFailed,
        RestoreFeedFailed,
        RestoreFeedSucceeded,
        UserReaction,
        TrendingTopic,
        TrendingTopicsRequestSucceeded,
        TrendingTopicsRequestFailed,
        SearchBy,
        ResetAiSucceeded;
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart'
    show EngineInitSucceeded, MapEvent;
import 'package:xayn_discovery_engine/src/discovery_engine_manager.dart'
    show DiscoveryEngineManager;
import 'package:xayn_discovery_engine/src/discovery_engine_worker.dart'
    as entry_point show main;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show AvailableSource, Source;
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
  final String? lastDbOverrideError;

  DiscoveryEngine._(this._manager, this.lastDbOverrideError);

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
  ///     feedMarket: {const FeedMarket(langCode: 'de', countryCode: 'DE')},
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
    String? deConfig,
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

      final initEvent = ClientEvent.init(configuration, deConfig: deConfig);
      final response = await manager.send(initEvent, timeout: null);
      await subscription?.cancel();

      if (response is! EngineInitSucceeded) {
        await manager.dispose();
        throw EngineInitException(
          'Something went wrong when sending over the configuration',
          response,
        );
      }

      return DiscoveryEngine._(manager, response.dbOverrideError);
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
  /// - [EngineExceptionRaised] indicating a failed operation, with a reason
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
  /// - [EngineExceptionRaised] for unexpected exception raised, with a reason
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
  /// - [EngineExceptionRaised] for unexpected exception raised, with a reason
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
  /// - [EngineExceptionRaised] indicating a failed operation, with a reason
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
  /// - [EngineExceptionRaised] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> addSourceToExcludedList(Source source) {
    return _trySend(() async {
      final event = ClientEvent.excludedSourceAdded(source);
      final response = await _manager.send(event);

      return response.mapEvent(
        addExcludedSourceRequestSucceeded: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Removes a source [Uri] from the set of excluded sources.
  ///
  /// In response it can return:
  /// - [ClientEventSucceeded] indicating a successful operation
  /// - [EngineExceptionRaised] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> removeSourceFromExcludedList(Source source) {
    return _trySend(() async {
      final event = ClientEvent.excludedSourceRemoved(source);
      final response = await _manager.send(event);

      return response.mapEvent(
        removeExcludedSourceRequestSucceeded: true,
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
  /// - [EngineExceptionRaised] indicating a failed operation, with a reason
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

  /// Overrides both trusted and excluded [Source]s which means provided sets
  /// will replace the current [Source]s.
  ///
  /// In response it can return:
  /// - [SetSourcesRequestSucceeded] indicating a successful operation,
  /// containing sets of both trusted and excluded [Source]s.
  /// - [SetSourcesRequestFailed] indicating a failed operation because of
  /// duplicates found in provided sets, containing a set of said duplicates.
  /// - [EngineExceptionRaised] indicating a failed operation, with a reason
  /// for such failure.
  //FIXME rename to setSourcePreferences when we move to message passing API,
  //      also rename add/remove trusted/excluded source to not contain the word
  //      list (it's two sets or more specific a map of source->preference)
  Future<EngineEvent> overrideSources({
    required Set<Source> trustedSources,
    required Set<Source> excludedSources,
  }) {
    return _trySend(() async {
      final response = await _manager.send(
        ClientEvent.setSourcesRequested(
          trustedSources: trustedSources,
          excludedSources: excludedSources,
        ),
      );

      return response.mapEvent(
        setSourcesRequestSucceeded: true,
        setSourcesRequestFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  Future<EngineEvent> addSourceToTrustedList(Source source) {
    return _trySend(() async {
      final event = ClientEvent.trustedSourceAdded(source);
      final response = await _manager.send(event);

      return response.mapEvent(
        addTrustedSourceRequestSucceeded: true,
        engineExceptionRaised: true,
      );
    });
  }

  Future<EngineEvent> removeSourceFromTrustedList(Source source) {
    return _trySend(() async {
      final event = ClientEvent.trustedSourceRemoved(source);
      final response = await _manager.send(event);

      return response.mapEvent(
        removeTrustedSourceRequestSucceeded: true,
        engineExceptionRaised: true,
      );
    });
  }

  Future<EngineEvent> getTrustedSourcesList() {
    return _trySend(() async {
      const event = ClientEvent.trustedSourcesListRequested();
      final response = await _manager.send(event);

      return response.mapEvent(
        trustedSourcesListRequestSucceeded: true,
        trustedSourcesListRequestFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Returns a list of [AvailableSource]s.
  ///
  /// In response it can return:
  /// - [AvailableSourcesListRequestSucceeded] indicating a successful operation,
  /// containing a set of available sources.
  /// - [AvailableSourcesListRequestFailed] indicating a failed operation
  /// - [EngineExceptionRaised] indicating a failed operation, with a reason
  /// for such failure.
  Future<EngineEvent> getAvailableSourcesList(String fuzzySearchTerm) {
    return _trySend(() async {
      final event = ClientEvent.availableSourcesListRequested(fuzzySearchTerm);
      final response = await _manager.send(event);

      return response.mapEvent(
        availableSourcesListRequestSucceeded: true,
        availableSourcesListRequestFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Logs the time in seconds spent by a user on a [Document] in a certain
  /// mode.
  ///
  /// In response it can return:
  /// - [ClientEventSucceeded] indicating a successful operation
  /// - [EngineExceptionRaised] indicating a failed operation, with a reason
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
  /// - [UserReaction.negative] indicates that the [Document] was **disliked**
  /// - [UserReaction.neutral] as a default **neutral** state of the [Document].
  ///
  /// In response it can return:
  /// - [ClientEventSucceeded] indicating a successful operation
  /// - [EngineExceptionRaised] indicating a failed operation, with a reason
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

  /// Requests a new active search for [Document]s related to `queryTerm`.
  ///
  /// In response it can return:
  /// - [ActiveSearchRequestSucceeded] for successful response, containing a list of
  /// [Document]s
  /// - [ActiveSearchRequestFailed] for failed response, with a reason for failure
  /// - [EngineExceptionRaised] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> requestQuerySearch(String queryTerm) {
    return _trySend(() async {
      final event =
          ClientEvent.activeSearchRequested(queryTerm, SearchBy.query);
      final response = await _manager.send(event);

      return response.mapEvent(
        activeSearchRequestSucceeded: true,
        activeSearchRequestFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Requests a new active search for [Document]s of a specific `topic`.
  ///
  /// In response it can return:
  /// - [ActiveSearchRequestSucceeded] for successful response, containing a list of
  /// [Document]s
  /// - [ActiveSearchRequestFailed] for failed response, with a reason for failure
  /// - [EngineExceptionRaised] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> requestTopicSearch(String topic) {
    return _trySend(() async {
      final event = ClientEvent.activeSearchRequested(topic, SearchBy.topic);
      final response = await _manager.send(event);

      return response.mapEvent(
        activeSearchRequestSucceeded: true,
        activeSearchRequestFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Requests next batch of [Document]s related to the current active search.
  ///
  /// In response it can return:
  /// - [NextActiveSearchBatchRequestSucceeded] for successful response, containing a list of
  /// [Document]s
  /// - [NextActiveSearchBatchRequestFailed] for failed response, with a reason for failure
  /// - [EngineExceptionRaised] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> requestNextActiveSearchBatch() {
    return _trySend(() async {
      const event = ClientEvent.nextActiveSearchBatchRequested();
      final response = await _manager.send(event);

      return response.mapEvent(
        nextActiveSearchBatchRequestSucceeded: true,
        nextActiveSearchBatchRequestFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Restores latest active search that wasn't closed.
  ///
  /// In response it can return:
  /// - [RestoreActiveSearchSucceeded] for successful response, containing a list of
  /// [Document]s
  /// - [RestoreActiveSearchFailed] for failed response, with a reason for failure
  /// - [EngineExceptionRaised] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> restoreActiveSearch() {
    return _trySend(() async {
      const event = ClientEvent.restoreActiveSearchRequested();
      final response = await _manager.send(event);

      return response.mapEvent(
        restoreActiveSearchSucceeded: true,
        restoreActiveSearchFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Returns the current active search term.
  ///
  /// In response it can return:
  /// - [ActiveSearchTermRequestSucceeded] for successful response, containing the
  /// search term
  /// - [ActiveSearchTermRequestFailed] for failed response, with a reason for failure
  /// - [EngineExceptionRaised] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> getActiveSearchTerm() {
    return _trySend(() async {
      const event = ClientEvent.activeSearchTermRequested();
      final response = await _manager.send(event);

      return response.mapEvent(
        activeSearchTermRequestSucceeded: true,
        activeSearchTermRequestFailed: true,
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
  /// - [ActiveSearchClosedSucceeded] indicating a successful operation
  /// - [ActiveSearchClosedFailed] indicating a failed operation, with a reason
  /// for such failure.
  /// - [EngineExceptionRaised] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> closeActiveSearch() {
    return _trySend(() async {
      const event = ClientEvent.activeSearchClosed();
      final response = await _manager.send(event);

      return response.mapEvent(
        activeSearchClosedSucceeded: true,
        activeSearchClosedFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Requests a new deep search for [Document]s related to a document.
  ///
  /// In response it can return:
  /// - [DeepSearchRequestSucceeded] for successful response, containing a list of
  /// [Document]s
  /// - [DeepSearchRequestFailed] for failed response, with a reason for failure
  /// - [EngineExceptionRaised] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> requestDeepSearch(DocumentId id) {
    return _trySend(() async {
      final event = ClientEvent.deepSearchRequested(id);
      final response = await _manager.send(event);

      return response.mapEvent(
        deepSearchRequestSucceeded: true,
        deepSearchRequestFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Requests for the current [TrendingTopic]s.
  ///
  /// In response it can return:
  /// - [TrendingTopicsRequestSucceeded] for successful response, containing a list of
  /// [TrendingTopic]s
  /// - [TrendingTopicsRequestFailed] for failed response, with a reason for failure
  /// - [EngineExceptionRaised] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> requestTrendingTopics() {
    return _trySend(() async {
      const event = ClientEvent.trendingTopicsRequested();
      final response = await _manager.send(event);

      return response.mapEvent(
        trendingTopicsRequestSucceeded: true,
        trendingTopicsRequestFailed: true,
        engineExceptionRaised: true,
      );
    });
  }

  /// Reset the state of the AI.
  ///
  /// This will not touch configurations like the
  /// market or trusted/excluded sources.
  ///
  /// In response it can return:
  /// - [ResetAiSucceeded] for successful response
  /// - [EngineExceptionRaised] for unexpected exception raised, with a reason
  /// for such failure.
  Future<EngineEvent> resetAi() {
    return _trySend(() async {
      const event = ClientEvent.resetAi();
      final response = await _manager.send(event);

      return response.mapEvent(
        resetAiSucceeded: true,
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
