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

import 'package:xayn_discovery_engine/src/api/events/client_events.dart'
    show SystemClientEvent;
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart'
    show EngineEvent, EngineExceptionReason;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarkets;

/// Business logic concerning the management of the engine system.
class SystemManager {
  final Engine _engine;

  SystemManager(this._engine);

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
      try {
        await _engine.setMarkets(feedMarkets);
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
    await _engine.resetAi();
    return const EngineEvent.resetAiSucceeded();
  }
}
