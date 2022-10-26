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

import 'dart:convert' show jsonEncode;

import 'package:xayn_discovery_engine/discovery_engine.dart'
    show cfgFeatureStorage;
import 'package:xayn_discovery_engine/src/api/events/client_events.dart'
    show SystemClientEvent;
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart'
    show EngineEvent, EngineExceptionReason;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarkets;
import 'package:xayn_discovery_engine/src/domain/models/history.dart';
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart'
    show SourceReacted;
import 'package:xayn_discovery_engine/src/domain/repository/document_repo.dart'
    show DocumentRepository;
import 'package:xayn_discovery_engine/src/domain/repository/source_reacted_repo.dart';

/// Business logic concerning the management of the engine system.
class SystemManager {
  final Engine _engine;
  final DocumentRepository _docRepo;
  final SourceReactedRepository _sourceReactedRepo;
  final Future<void> Function() _clearAiState;

  SystemManager(
    this._engine,
    this._docRepo,
    this._sourceReactedRepo,
    this._clearAiState,
  );

  /// Handle the given system client event.
  ///
  /// Fails if [event] does not have a handler implemented.
  Future<EngineEvent> handleSystemClientEvent(SystemClientEvent event) =>
      event.maybeWhen(
        configurationChanged: changeConfiguration,
        resetAi: resetAi,
        orElse: () =>
            throw UnimplementedError('handler not implemented for $event'),
      );

  /// Changes the configuration of the engine system.
  Future<EngineEvent> changeConfiguration(
    FeedMarkets? feedMarkets,
    int? maxItemsPerFeedBatch,
    int? maxItemsPerSearchBatch,
  ) async {
    if (feedMarkets != null) {
      final history = cfgFeatureStorage
          ? <HistoricDocument>[] // unused
          : await _docRepo.fetchHistory();
      final sources = cfgFeatureStorage
          ? <SourceReacted>[] // unused
          : await _sourceReactedRepo.fetchAll();
      try {
        await _engine.setMarkets(history, sources, feedMarkets);
      } catch (e, st) {
        return EngineEvent.engineExceptionRaised(
          EngineExceptionReason.genericError,
          message: '$e',
          stackTrace: '$st',
        );
      }
    }
    final deConfig = <String, dynamic>{};
    if (maxItemsPerFeedBatch != null) {
      deConfig['feed'] = {'max_docs_per_batch': maxItemsPerFeedBatch};
    }
    if (maxItemsPerSearchBatch != null) {
      deConfig['search'] = {'max_docs_per_batch': maxItemsPerSearchBatch};
    }
    await _engine.configure(jsonEncode(deConfig));

    return const EngineEvent.clientEventSucceeded();
  }

  Future<EngineEvent> resetAi() async {
    await _clearAiState();
    await _engine.resetAi();
    return const EngineEvent.resetAiSucceeded();
  }
}
