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

import 'dart:ffi' show nullptr, Pointer;
import 'dart:typed_data' show Uint8List;

import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show DocumentWithActiveData;
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/domain/models/time_spent.dart'
    show TimeSpent;
import 'package:xayn_discovery_engine/src/domain/models/user_reacted.dart'
    show UserReacted;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustResultSharedEngineString, RustSharedEngine;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show asyncFfi, ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/document/document_vec.dart'
    show DocumentSliceFfi;
import 'package:xayn_discovery_engine/src/ffi/types/document/time_spent.dart'
    show TimeSpentFfi;
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reacted.dart'
    show UserReactedFfi;
import 'package:xayn_discovery_engine/src/ffi/types/feed_market_vec.dart'
    show FeedMarketSliceFfi;
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
class DiscoveryEngineFfi {
  final Boxed<RustResultSharedEngineString> _boxedResult;
  Pointer<RustSharedEngine> _sharedEngine;

  DiscoveryEngineFfi._(final this._boxedResult, final this._sharedEngine);

  /// Initializes the engine.
  static Future<DiscoveryEngineFfi> initialize(
    final Configuration config,
    final NativeSetupData setupData, [
    final Uint8List? state,
  ]) async {
    final boxedConfig = InitConfigFfi(config, setupData).allocNative();
    final boxedState = state?.allocNative();

    final boxedResult = Boxed(
      await asyncFfi.initialize(
        boxedConfig.move(),
        boxedState?.move() ?? nullptr,
      ),
      ffi.drop_result_shared_engine_string,
    );
    final Pointer<RustSharedEngine> sharedEngine;
    try {
      sharedEngine =
          resultSharedEngineStringFfiAdapter.readNative(boxedResult.mut);
    } catch (_) {
      boxedResult.free();
      rethrow;
    }

    return DiscoveryEngineFfi._(boxedResult, sharedEngine);
  }

  /// Serializes the engine.
  Future<Uint8List> serialize() async {
    final result = await asyncFfi.serialize(_sharedEngine);

    return resultVecU8StringFfiAdapter.consumeNative(result);
  }

  /// Sets the markets.
  Future<void> setMarkets(final List<FeedMarket> markets) async {
    final boxedMarkets = markets.allocVec();
    final result =
        await asyncFfi.setMarkets(_sharedEngine, boxedMarkets.move());

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  /// Gets feed documents.
  Future<List<DocumentWithActiveData>> getFeedDocuments(
    final int maxDocuments,
  ) async {
    final result = await asyncFfi.getFeedDocuments(_sharedEngine, maxDocuments);

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData();
  }

  /// Processes time spent.
  Future<void> timeSpent(final TimeSpent timeSpent) async {
    final boxedTimeSpent = timeSpent.allocNative();
    final result =
        await asyncFfi.timeSpent(_sharedEngine, boxedTimeSpent.move());

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  /// Processes user reaction.
  Future<void> userReacted(final UserReacted userReacted) async {
    final boxedUserReacted = userReacted.allocNative();
    final result =
        await asyncFfi.userReacted(_sharedEngine, boxedUserReacted.move());

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  /// Drops the engine.
  ///
  /// # Safety
  /// Must only be called after all other futures of the engine have been completed.
  void free() {
    _sharedEngine = nullptr;
    _boxedResult.free();
  }
}
