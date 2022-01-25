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

import 'dart:async' show Completer;
import 'dart:typed_data' show Uint8List;
import 'package:hive/hive.dart' show Hive;
import 'package:xayn_discovery_engine/src/api/api.dart'
    show
        ClientEvent,
        DocumentClientEvent,
        EngineEvent,
        EngineExceptionReason,
        FeedClientEvent,
        Init;
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
import 'package:xayn_discovery_engine/src/domain/models/web_resource.dart'
    show WebResourceAdapter;
import 'package:xayn_discovery_engine/src/domain/models/web_resource_provider.dart'
    show WebResourceProviderAdapter;
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
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_document_id_adapter.dart'
    show DocumentIdAdapter;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_uri_adapter.dart'
    show UriAdapter;
import 'package:xayn_discovery_engine/src/logger.dart' show logger;

class EventHandler {
  // core engine
  final _engineFuture = Completer<Engine>();
  // repositories
  late final DocumentRepository _documentRepository;
  late final ActiveDocumentDataRepository _activeDataRepository;
  late final ChangedDocumentRepository _changedDocumentRepository;
  // managers
  late final DocumentManager _documentManager;
  late final FeedManager _feedManager;

  Future<EngineEvent> _initEngine(Configuration config) async {
    try {
      // init hive
      // TODO: fix Firefox incognito issue
      Hive.init('${config.applicationDirectoryPath}/database');
      // register hive adapters
      Hive.registerAdapter(DocumentAdapter());
      Hive.registerAdapter(DocumentFeedbackAdapter());
      Hive.registerAdapter(ActiveDocumentDataAdapter());
      Hive.registerAdapter(WebResourceAdapter());
      Hive.registerAdapter(WebResourceProviderAdapter());
      Hive.registerAdapter(DocumentIdAdapter());
      Hive.registerAdapter(UriAdapter());

      // open boxes
      await Hive.openBox<Document>(documentBox);
      await Hive.openBox<ActiveDocumentData>(activeDocumentDataBox);
      await Hive.openBox<Uint8List>(changedDocumentIdBox);

      // create repositories
      _documentRepository = HiveDocumentRepository();
      _activeDataRepository = HiveActiveDocumentDataRepository();
      _changedDocumentRepository = HiveChangedDocumentRepository();

      // fetch AI assets
      final assetFetcher = HttpAssetFetcher(config.assetsUrl);
      final manifestReader = createManifestReader();
      final dataProvider = createDataProvider(
        assetFetcher,
        manifestReader,
        config.applicationDirectoryPath,
      );
      await dataProvider.getSetupData();
      // TODO: replace with real engine and pass in setup data
      final engine = MockEngine();

      // init managers
      _documentManager = DocumentManager(
        _documentRepository,
        _activeDataRepository,
        _changedDocumentRepository,
      );
      _feedManager =
          FeedManager(_documentManager, engine, config.maxItemsPerFeedBatch);

      // complete future
      _engineFuture.complete(engine);

      return const EngineEvent.clientEventSucceeded();
    } catch (e) {
      logger.e(e);
      return const EngineEvent.engineExceptionRaised(
        // TODO: add dedicated variants
        EngineExceptionReason.genericError,
      );
    }
  }

  Future<EngineEvent> handleMessage(ClientEvent clientEvent) async {
    if (clientEvent is Init) {
      return _initEngine(clientEvent.configuration);
    }

    if (!_engineFuture.isCompleted) {
      return const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.engineNotReady,
      );
    }

    // alwas wait for the engien to be ready before handling the next event
    await _engineFuture.future;

    // prepare reposnses
    EngineEvent response = const EngineEvent.clientEventSucceeded();

    try {
      if (clientEvent is FeedClientEvent) {
        // TODO: reassign response to whatever FeedManager returns
        await _feedManager.handleFeedClientEvent(clientEvent);
      } else if (clientEvent is DocumentClientEvent) {
        await _documentManager.handleDocumentClientEvent(clientEvent);
      }
    } catch (e) {
      response = const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.genericError,
      );
    }

    return response;
  }
}
