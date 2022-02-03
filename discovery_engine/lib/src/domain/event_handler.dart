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
import 'package:hive/hive.dart' show Hive;
import 'package:xayn_discovery_engine/src/api/api.dart'
    show
        ClientEvent,
        DocumentClientEvent,
        EngineEvent,
        EngineExceptionRaised,
        EngineExceptionReason,
        FeedClientEvent,
        Init,
        SystemClientEvent;
import 'package:xayn_discovery_engine/src/domain/assets/assets.dart'
    show AssetFetcherException, SetupData;
import 'package:xayn_discovery_engine/src/domain/document_manager.dart'
    show DocumentManager;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/engine/mock_engine.dart'
    show MockEngine;
import 'package:xayn_discovery_engine/src/domain/feed_manager.dart'
    show FeedManager;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData, ActiveDocumentDataAdapter;
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, DocumentAdapter, DocumentFeedbackAdapter;
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResourceAdapter;
import 'package:xayn_discovery_engine/src/domain/repository/active_document_repo.dart'
    show ActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/domain/repository/changed_document_repo.dart'
    show ChangedDocumentRepository;
import 'package:xayn_discovery_engine/src/domain/repository/document_repo.dart'
    show DocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/assets/assets.dart'
    show createDataProvider, createManifestReader;
import 'package:xayn_discovery_engine/src/infrastructure/assets/http_asset_fetcher.dart'
    show HttpAssetFetcher;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show activeDocumentDataBox, changedDocumentIdBox, documentBox;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart'
    show HiveActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_changed_document_repo.dart'
    show HiveChangedDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_document_repo.dart'
    show HiveDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_duration_adapter.dart'
    show DurationAdapter;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_unique_id_adapter.dart'
    show DocumentIdAdapter, StackIdAdapter;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_uri_adapter.dart'
    show UriAdapter;
import 'package:xayn_discovery_engine/src/logger.dart' show logger;

const kEnginePath = 'engine';
const kDatabasePath = 'database';

class EventHandler {
  Future<Engine>? _engineFuture;
  late final DocumentRepository _documentRepository;
  late final ActiveDocumentDataRepository _activeDataRepository;
  late final ChangedDocumentRepository _changedDocumentRepository;
  late final DocumentManager _documentManager;
  late final FeedManager _feedManager;

  /// Decides what to do with incoming [ClientEvent] by passing it
  /// to a dedicated manager and returns the appropriate response in the form
  /// of a proper [EngineEvent].
  ///
  /// This handler is invoked by a [ClientEvent]s stream listener, so it is
  /// called each time there is a new event on the stream, without waiting for
  /// previous events to finish processing.
  Future<EngineEvent> handleMessage(ClientEvent clientEvent) async {
    if (clientEvent is Init) {
      try {
        _engineFuture = _initEngine(clientEvent.configuration);
        await _engineFuture;
        return const EngineEvent.clientEventSucceeded();
      } catch (e) {
        logger.e(e);
        _engineFuture = null;
        final reason = e is AssetFetcherException
            ? EngineExceptionReason.failedToGetAssets
            : EngineExceptionReason.genericError;
        return EngineExceptionRaised(reason);
      }
    }

    if (_engineFuture == null) {
      return const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.engineNotReady,
      );
    }

    try {
      await _engineFuture;
    } catch (_) {
      return const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.engineNotReady,
      );
    }

    // prepare responses
    EngineEvent response = const EngineEvent.clientEventSucceeded();

    try {
      if (clientEvent is FeedClientEvent) {
        response = await _feedManager.handleFeedClientEvent(clientEvent);
      } else if (clientEvent is DocumentClientEvent) {
        await _documentManager.handleDocumentClientEvent(clientEvent);
      } else if (clientEvent is SystemClientEvent) {
        // TODO: we need to handle other system events (reset and config changed)
      } else {
        response = const EngineEvent.engineExceptionRaised(
          EngineExceptionReason.wrongEventRequested,
        );
      }
    } catch (e) {
      // log the error
      logger.e(e);

      response = const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.genericError,
      );
    }

    return response;
  }

  Future<Engine> _initEngine(Configuration config) async {
    // init hive
    await _initDatabase(config.applicationDirectoryPath);

    // create repositories
    _documentRepository = HiveDocumentRepository();
    _activeDataRepository = HiveActiveDocumentDataRepository();
    _changedDocumentRepository = HiveChangedDocumentRepository();

    // fetch AI assets
    await _fetchAssets(config);

    // init the engine
    // TODO: replace with real engine and pass in setup data
    final engine = MockEngine();

    // init managers
    _documentManager = DocumentManager(
      engine,
      _documentRepository,
      _activeDataRepository,
    );
    _feedManager = FeedManager(
      engine,
      config.maxItemsPerFeedBatch,
      _documentRepository,
      _activeDataRepository,
      _changedDocumentRepository,
    );

    return engine;
  }

  Future<SetupData> _fetchAssets(Configuration config) async {
    final appDir = config.applicationDirectoryPath;
    final storageDirPath = '$appDir/$kEnginePath';
    final assetFetcher = HttpAssetFetcher(config.assetsUrl);
    final manifestReader = createManifestReader();
    final dataProvider = createDataProvider(
      assetFetcher,
      manifestReader,
      storageDirPath,
    );
    return dataProvider.getSetupData();
  }

  Future<void> _initDatabase(String appDir) async {
    Hive.init('$appDir/$kEnginePath/$kDatabasePath');
    // register hive adapters
    Hive.registerAdapter(DocumentAdapter());
    Hive.registerAdapter(DocumentFeedbackAdapter());
    Hive.registerAdapter(ActiveDocumentDataAdapter());
    Hive.registerAdapter(NewsResourceAdapter());
    Hive.registerAdapter(DocumentIdAdapter());
    Hive.registerAdapter(StackIdAdapter());
    Hive.registerAdapter(DurationAdapter());
    Hive.registerAdapter(UriAdapter());

    // open boxes
    await _openDbBox<Document>(documentBox);
    await _openDbBox<ActiveDocumentData>(activeDocumentDataBox);
    await _openDbBox<Uint8List>(changedDocumentIdBox);
  }

  /// Tries to open a box persisted on disk. In case of failure opens it in memory.
  Future<void> _openDbBox<T>(String name) async {
    try {
      await Hive.openBox<T>(name);
    } catch (e) {
      /// Some browsers (e.g. Firefox) are not allowing the use of IndexedDB
      /// in `Private Mode`, so we need to use Hive in-memory instead
      await Hive.openBox<T>(name, bytes: Uint8List(0));
    }
  }
}
