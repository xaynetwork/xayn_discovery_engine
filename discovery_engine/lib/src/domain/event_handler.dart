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

import 'package:async/async.dart' show StreamGroup;

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
    show AssetFetcherException, AssetReporter, SetupData, kAssetsPath;
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
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show mockedAvailableSources;
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
import 'package:xayn_discovery_engine/src/infrastructure/migration.dart';
import 'package:xayn_discovery_engine/src/logger.dart' show logger;

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
          deConfig: clientEvent.deConfig,
        );
      } catch (e, st) {
        logger.e('failed to initialize the engine', e, st);
        final EngineExceptionReason reason;
        if (e is AssetFetcherException) {
          reason = EngineExceptionReason.failedToGetAssets;
        } else if (e is InvalidEngineStateException) {
          reason = EngineExceptionReason.invalidEngineState;
        } else {
          reason = EngineExceptionReason.genericError;
        }
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
    String? deConfig,
  }) async {
    final setupData = await _fetchAssets(config);
    final availableSources = config.isMocked()
        ? mockedAvailableSources
        : await setupData.getAvailableSources();
    final dartMigrationData = await DartMigrationData.fromRepositories(
      config.applicationDirectoryPath,
    );

    final initializer = EngineInitializer(
      config: config,
      setupData: setupData,
      deConfig: deConfig,
      dartMigrationData: dartMigrationData,
    );
    _engine = config.isMocked()
        ? MockEngine(initializer)
        : await DiscoveryEngineFfi.initialize(initializer);

    _documentManager = DocumentManager(_engine!, _changedDocumentsReporter);
    _feedManager = FeedManager(_engine!, availableSources);
    _searchManager = SearchManager(_engine!);
    _systemManager = SystemManager(_engine!);

    return EngineEvent.engineInitSucceeded(_engine!.lastDbOverrideError);
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
}

/// Thrown when the engine cannot be recovered from the given state.
class InvalidEngineStateException implements Exception {
  final String message;

  InvalidEngineStateException(this.message);

  @override
  String toString() =>
      'InvalidEngineStateException: $message. Try to restart the engine.';
}
