// Copyright 2022 Xayn AG
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

import 'dart:typed_data' show Uint8List;
import 'package:async/async.dart' show StreamGroup;
import 'package:hive/hive.dart' show Hive;
import 'package:meta/meta.dart' show visibleForTesting;
import 'package:xayn_discovery_engine/src/api/api.dart'
    show
        ClientEvent,
        DocumentClientEvent,
        EngineEvent,
        EngineExceptionReason,
        FeedClientEvent,
        Init,
        SearchClientEvent,
        SystemClientEvent;
import 'package:xayn_discovery_engine/src/domain/assets/assets.dart'
    show
        AssetFetcherException,
        AssetReporter,
        SetupData,
        kAssetsPath,
        kDatabasePath;
import 'package:xayn_discovery_engine/src/domain/changed_documents_reporter.dart'
    show ChangedDocumentsReporter;
import 'package:xayn_discovery_engine/src/domain/document_manager.dart'
    show DocumentManager;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine, EngineInitializer;
import 'package:xayn_discovery_engine/src/domain/engine/mock_engine.dart'
    show MockEngine;
import 'package:xayn_discovery_engine/src/domain/feed_manager.dart'
    show FeedManager;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData, ActiveDocumentDataAdapter;
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    show ActiveSearch, ActiveSearchAdapter, SearchByAdapter;
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, DocumentAdapter, UserReactionAdapter;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarketAdapter;
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResourceAdapter;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show AvailableSources, Source;
import 'package:xayn_discovery_engine/src/domain/models/source_preference.dart'
    show SourcePreference, SourcePreferenceAdapter, PreferenceModeAdapter;
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart'
    show DocumentViewModeAdapter;
import 'package:xayn_discovery_engine/src/domain/search_manager.dart'
    show SearchManager;
import 'package:xayn_discovery_engine/src/domain/system_manager.dart'
    show SystemManager;
import 'package:xayn_discovery_engine/src/ffi/types/engine.dart'
    show DiscoveryEngineFfi;
import 'package:xayn_discovery_engine/src/infrastructure/assets/assets.dart'
    show createDataProvider;
import 'package:xayn_discovery_engine/src/infrastructure/assets/http_asset_fetcher.dart'
    show HttpAssetFetcher;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show
        activeDocumentDataBox,
        documentBox,
        engineStateBox,
        excludedSourcesBox,
        trustedSourcesBox,
        searchBox,
        sourcePreferenceBox;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart'
    show HiveActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_search_repo.dart'
    show HiveActiveSearchRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_document_repo.dart'
    show HiveDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_engine_state_repo.dart'
    show HiveEngineStateRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_source_preference_repo.dart'
    show HiveSourcePreferenceRepository;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_duration_adapter.dart'
    show DurationAdapter;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_embedding_adapter.dart'
    show EmbeddingAdapter;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_source_adapter.dart'
    show SetSourceAdapter, SourceAdapter;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_unique_id_adapter.dart'
    show DocumentIdAdapter, StackIdAdapter;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_uri_adapter.dart'
    show UriAdapter;
import 'package:xayn_discovery_engine/src/logger.dart' show logger;

class EventConfig {
  int maxFeedDocs;
  int maxSearchDocs;

  EventConfig({
    required this.maxFeedDocs,
    required this.maxSearchDocs,
  })  : assert(maxFeedDocs > 0),
        assert(maxSearchDocs > 0);
}

class EventHandler {
  Engine? _engine;
  final AssetReporter _assetReporter;
  final ChangedDocumentsReporter _changedDocumentsReporter;
  late final DocumentManager _documentManager;
  late final FeedManager _feedManager;
  late final SearchManager _searchManager;
  late final SystemManager _systemManager;

  EventHandler()
      : _engine = null,
        _assetReporter = AssetReporter(),
        _changedDocumentsReporter = ChangedDocumentsReporter();

  Stream<EngineEvent> get events => StreamGroup.mergeBroadcast([
        _assetReporter.progress,
        _changedDocumentsReporter.changedDocuments,
      ]);

  /// Performs clean-up. Closes all open database boxes and stream controllers.
  Future<void> close() async {
    final engine = _engine;
    _engine = null;
    await engine?.dispose();
    await _changedDocumentsReporter.close();
    await Hive.close();
  }

  /// Decides what to do with incoming [ClientEvent] by passing it
  /// to a dedicated manager and returns the appropriate response in the form
  /// of a proper [EngineEvent].
  ///
  /// This handler is invoked by a [ClientEvent]s stream listener, so it is
  /// called each time there is a new event on the stream, without waiting for
  /// previous events to finish processing.
  Future<EngineEvent> handleMessage(ClientEvent clientEvent) async {
    if (clientEvent is Init) {
      if (_engine != null) {
        return const EngineEvent.engineExceptionRaised(
          EngineExceptionReason.wrongEventRequested,
        );
      }

      try {
        return await _initEngine(
          clientEvent.configuration,
          aiConfig: clientEvent.aiConfig,
        );
      } catch (e, st) {
        logger.e('failed to initialize the engine', e, st);
        final reason = e is AssetFetcherException
            ? EngineExceptionReason.failedToGetAssets
            : EngineExceptionReason.genericError;
        return EngineEvent.engineExceptionRaised(
          reason,
          message: '$e',
          stackTrace: '$st',
        );
      }
    }

    if (_engine == null) {
      return const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.engineNotReady,
      );
    }

    try {
      if (clientEvent is FeedClientEvent) {
        return await _feedManager.handleFeedClientEvent(clientEvent);
      } else if (clientEvent is DocumentClientEvent) {
        await _documentManager.handleDocumentClientEvent(clientEvent);
        return const EngineEvent.clientEventSucceeded();
      } else if (clientEvent is SystemClientEvent) {
        return await _systemManager.handleSystemClientEvent(clientEvent);
      } else if (clientEvent is SearchClientEvent) {
        return await _searchManager.handleSearchClientEvent(clientEvent);
      } else {
        return const EngineEvent.engineExceptionRaised(
          EngineExceptionReason.wrongEventRequested,
        );
      }
    } catch (e, st) {
      // log the error
      logger.e('Handling ClientEvent by one of the managers failed', e, st);

      return EngineEvent.engineExceptionRaised(
        EngineExceptionReason.genericError,
        message: '$e',
        stackTrace: '$st',
      );
    }
  }

  Future<EngineEvent> _initEngine(
    Configuration config, {
    String? aiConfig,
  }) async {
    // init hive
    registerHiveAdapters();
    await initDatabase(config.applicationDirectoryPath);

    // create repositories
    final documentRepository = HiveDocumentRepository();
    final activeDataRepository = HiveActiveDocumentDataRepository();
    final activeSearchRepository = HiveActiveSearchRepository();
    final engineStateRepository = HiveEngineStateRepository();
    final sourcePreferenceRepository = HiveSourcePreferenceRepository();

    final setupData = await _fetchAssets(config);
    final engineState = await engineStateRepository.load();
    final history = await documentRepository.fetchHistory();
    final trustedSources = await sourcePreferenceRepository.getTrusted();
    final excludedSources = await sourcePreferenceRepository.getExcluded();
    final availableSources = AvailableSources([]); // TODO: TY-2746

    final engine = await _initializeEngine(
      EngineInitializer(
        config: config,
        setupData: setupData,
        engineState: engineState,
        history: history,
        aiConfig: aiConfig,
        trustedSources: trustedSources,
        excludedSources: excludedSources,
      ),
    );

    // init managers
    final eventConfig = EventConfig(
      maxFeedDocs: config.maxItemsPerFeedBatch,
      maxSearchDocs: config.maxItemsPerSearchBatch,
    );
    _documentManager = DocumentManager(
      engine,
      documentRepository,
      activeDataRepository,
      engineStateRepository,
      _changedDocumentsReporter,
    );
    _feedManager = FeedManager(
      engine,
      eventConfig,
      documentRepository,
      activeDataRepository,
      engineStateRepository,
      sourcePreferenceRepository,
      availableSources,
    );
    _searchManager = SearchManager(
      engine,
      eventConfig,
      activeSearchRepository,
      documentRepository,
      activeDataRepository,
      engineStateRepository,
    );
    _systemManager = SystemManager(engine, eventConfig, documentRepository);

    _engine = engine;

    return const EngineEvent.clientEventSucceeded();
  }

  Future<Engine> _initializeEngine(EngineInitializer initializer) async {
    if (initializer.config.apiKey == 'use-mock-engine' &&
        initializer.config.apiBaseUrl == 'https://use-mock-engine.test') {
      return MockEngine(initializer);
    }
    return DiscoveryEngineFfi.initialize(initializer);
  }

  Future<SetupData> _fetchAssets(Configuration config) async {
    final appDir = config.applicationDirectoryPath;
    final storageDirPath = '$appDir/$kAssetsPath';
    final assetFetcher = HttpAssetFetcher(config.assetsUrl);
    final dataProvider = createDataProvider(
      assetFetcher,
      _assetReporter,
      storageDirPath,
    );
    return dataProvider.getSetupData(config.manifest);
  }

  @visibleForTesting
  static void registerHiveAdapters() {
    Hive.registerAdapter(DocumentAdapter());
    Hive.registerAdapter(UserReactionAdapter());
    Hive.registerAdapter(DocumentViewModeAdapter());
    Hive.registerAdapter(ActiveDocumentDataAdapter());
    Hive.registerAdapter(NewsResourceAdapter());
    Hive.registerAdapter(DocumentIdAdapter());
    Hive.registerAdapter(StackIdAdapter());
    Hive.registerAdapter(DurationAdapter());
    Hive.registerAdapter(UriAdapter());
    Hive.registerAdapter(EmbeddingAdapter());
    Hive.registerAdapter(FeedMarketAdapter());
    Hive.registerAdapter(SearchByAdapter());
    Hive.registerAdapter(ActiveSearchAdapter());
    Hive.registerAdapter(SourceAdapter());
    Hive.registerAdapter(SetSourceAdapter());
    Hive.registerAdapter(SourcePreferenceAdapter());
    Hive.registerAdapter(PreferenceModeAdapter());
  }

  @visibleForTesting
  static Future<void> initDatabase(String appDir) async {
    Hive.init('$appDir/$kDatabasePath');

    // open boxes
    await Future.wait([
      _openDbBox<Document>(documentBox),
      _openDbBox<ActiveDocumentData>(activeDocumentDataBox),

      /// See TY-2799
      /// Hive usually compacts our boxes automatically. However, with the default
      /// strategy, compaction is triggered after 60 deleted entries. This leads
      /// to the problem that our engine state is constantly growing because we
      /// are only overwriting it and not deleting it. Therefore we call it with
      /// `compact: true`.
      _openDbBox<Uint8List>(engineStateBox, compact: true),
      _openDbBox<ActiveSearch>(searchBox),
      _openDbBox<Set<Source>>(trustedSourcesBox),
      _openDbBox<Set<Source>>(excludedSourcesBox),
      _openDbBox<SourcePreference>(sourcePreferenceBox),
    ]);
  }

  /// Tries to open a box persisted on disk. In case of failure opens it in memory.
  /// If `compact` is set to `true`, compaction of the box will be triggered
  /// after opening.
  static Future<void> _openDbBox<T>(String name, {bool compact = false}) async {
    try {
      final box = await Hive.openBox<T>(name);

      if (compact) {
        await box.compact();
      }
    } catch (e) {
      /// Some browsers (e.g. Firefox) are not allowing the use of IndexedDB
      /// in `Private Mode`, so we need to use Hive in-memory instead
      await Hive.openBox<T>(name, bytes: Uint8List(0));
    }
  }
}
