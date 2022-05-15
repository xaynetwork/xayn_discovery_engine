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

import 'package:meta/meta.dart' show visibleForTesting;
import 'package:xayn_discovery_engine/src/api/events/client_events.dart'
    show SystemClientEvent;
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart'
    show EngineEvent, EngineExceptionReason;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/event_handler.dart'
    show EventConfig;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarkets;
import 'package:xayn_discovery_engine/src/domain/repository/document_repo.dart'
    show DocumentRepository;

/// Business logic concerning the management of the engine system.
class SystemManager {
  final Engine _engine;
  final EventConfig _config;
  final DocumentRepository _docRepo;

  SystemManager(this._engine, this._config, this._docRepo);

  @visibleForTesting
  int get maxFeedDocs => _config.maxFeedDocs;

  @visibleForTesting
  int get maxSearchDocs => _config.maxSearchDocs;

  /// Handle the given system client event.
  ///
  /// Fails if [event] does not have a handler implemented.
  Future<EngineEvent> handleSystemClientEvent(SystemClientEvent event) =>
      event.maybeWhen(
        configurationChanged: changeConfiguration,
        resetAI: resetAI,
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
      final history = await _docRepo.fetchHistory();
      try {
        await _engine.setMarkets(history, feedMarkets);
      } catch (e, st) {
        return EngineEvent.engineExceptionRaised(
          EngineExceptionReason.genericError,
          message: '$e',
          stackTrace: '$st',
        );
      }
    }
    if (maxItemsPerFeedBatch != null) {
      _config.maxFeedDocs = maxItemsPerFeedBatch;
    }
    if (maxItemsPerSearchBatch != null) {
      _config.maxSearchDocs = maxItemsPerSearchBatch;
    }

    return const EngineEvent.clientEventSucceeded();
  }

  Future<EngineEvent> resetAI() async {
    // TODO implement
    //  - check all repositories and clear all which need clearing
    //  - call rust engine reset
    return const EngineEvent.resetAISucceeded();
  }
}
