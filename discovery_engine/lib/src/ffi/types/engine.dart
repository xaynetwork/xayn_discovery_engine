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

import 'dart:ffi' show nullptr;
import 'dart:typed_data' show Uint8List;

import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine, EngineInitializer;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show DocumentWithActiveData;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarkets;
import 'package:xayn_discovery_engine/src/domain/models/history.dart'
    show HistoricDocument;
import 'package:xayn_discovery_engine/src/domain/models/time_spent.dart'
    show TimeSpent;
import 'package:xayn_discovery_engine/src/domain/models/user_reacted.dart'
    show UserReacted;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustSharedEngine;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show asyncFfi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/document/document_vec.dart'
    show DocumentSliceFfi;
import 'package:xayn_discovery_engine/src/ffi/types/document/time_spent.dart'
    show TimeSpentFfi;
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reacted.dart'
    show UserReactedFfi;
import 'package:xayn_discovery_engine/src/ffi/types/feed_market_vec.dart'
    show FeedMarketSliceFfi;
import 'package:xayn_discovery_engine/src/ffi/types/history.dart'
    show HistoricDocumentVecFfi;
import 'package:xayn_discovery_engine/src/ffi/types/init_config.dart'
    show InitConfigFfi;
import 'package:xayn_discovery_engine/src/ffi/types/primitives.dart'
    show Uint8ListFfi;
import 'package:xayn_discovery_engine/src/ffi/types/result.dart'
    show
        resultSharedEngineStringFfiAdapter,
        resultVecDocumentStringFfiAdapter,
        resultVecU8StringFfiAdapter,
        resultVoidStringFfiAdapter;
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/data_provider.dart'
    show NativeSetupData;

/// A handle to the discovery engine.
class DiscoveryEngineFfi implements Engine {
  final Boxed<RustSharedEngine> _engine;

  const DiscoveryEngineFfi._(final this._engine);

  /// Initializes the engine.
  static Future<DiscoveryEngineFfi> initialize(
    EngineInitializer initializer,
  ) async {
    final setupData = initializer.setupData;
    if (setupData is! NativeSetupData) {
      throw ArgumentError.value(
        setupData,
        'setupData',
        'must be NativeSetupData',
      );
    }
    final result = await asyncFfi.initialize(
      InitConfigFfi(
        initializer.config,
        setupData,
        aiConfig: initializer.aiConfig,
      ).allocNative().move(),
      initializer.engineState?.allocNative().move() ?? nullptr,
      initializer.history.allocNative().move(),
    );
    final boxedEngine = resultSharedEngineStringFfiAdapter.moveNative(result);
    return DiscoveryEngineFfi._(boxedEngine);
  }

  /// Serializes the engine.
  @override
  Future<Uint8List> serialize() async {
    final result = await asyncFfi.serialize(_engine.ref);

    return resultVecU8StringFfiAdapter.consumeNative(result);
  }

  /// Sets the markets.
  @override
  Future<void> setMarkets(
    final List<HistoricDocument> history,
    final FeedMarkets markets,
  ) async {
    final result = await asyncFfi.setMarkets(
      _engine.ref,
      markets.toList().allocVec().move(),
      history.allocNative().move(),
    );

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  /// Gets feed documents.
  @override
  Future<List<DocumentWithActiveData>> getFeedDocuments(
    final List<HistoricDocument> history,
    final int maxDocuments,
  ) async {
    final result = await asyncFfi.getFeedDocuments(
      _engine.ref,
      history.allocNative().move(),
      maxDocuments,
    );

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData();
  }

  /// Processes time spent.
  @override
  Future<void> timeSpent(final TimeSpent timeSpent) async {
    final boxedTimeSpent = timeSpent.allocNative();
    final result = await asyncFfi.timeSpent(_engine.ref, boxedTimeSpent.move());

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  /// Processes user reaction.
  ///
  /// The history is only required for positive reactions.
  @override
  Future<void> userReacted(
    final List<HistoricDocument>? history,
    final UserReacted userReacted,
  ) async {
    final boxedUserReacted = userReacted.allocNative();
    final result = await asyncFfi.userReacted(
      _engine.ref,
      history?.allocNative().move() ?? nullptr,
      boxedUserReacted.move(),
    );

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  /// Drops the engine.
  ///
  /// # Safety
  /// Must only be called after all other futures of the engine have been completed.
  void free() {
    _engine.free();
  }
}
