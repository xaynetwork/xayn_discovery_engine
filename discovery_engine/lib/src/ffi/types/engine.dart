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
import 'dart:typed_data' show Float32List, Uint8List;

import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustEngine, RustResultSharedEngineString;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart'
    show asyncCore, ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/document/document.dart'
    show DocumentFfi;
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
class DiscoveryEngine {
  final Boxed<RustResultSharedEngineString> _boxedResult;
  Pointer<RustEngine> _sharedEngine;

  DiscoveryEngine._(this._boxedResult, this._sharedEngine);

  /// Initializes the engine.
  static Future<DiscoveryEngine> initialize(
    final Configuration config,
    final NativeSetupData setupData, [
    final Uint8List? state,
  ]) async {
    final boxedConfig = InitConfigFfi(config, setupData).allocNative();
    final boxedState = state?.allocNative();

    final boxedResult = Boxed(
      await asyncCore.initialize(
        boxedConfig.move(),
        boxedState?.move() ?? nullptr,
      ),
      ffi.drop_result_shared_engine_string,
    );
    final Pointer<RustEngine> sharedEngine;
    try {
      sharedEngine = resultSharedEngineStringFfiAdapter.readNative(
        boxedResult.mut,
        mapErr: (error) => Exception(error),
      );
    } catch (_) {
      boxedResult.free();
      rethrow;
    }

    return DiscoveryEngine._(boxedResult, sharedEngine);
  }

  /// Serializes the engine.
  Future<Uint8List> serialize() async {
    final boxedResult = Boxed(
      await asyncCore.serialize(_sharedEngine),
      ffi.drop_result_vec_u8_string,
    );

    return resultVecU8StringFfiAdapter.consumeNative(
      boxedResult,
      mapErr: (error) => Exception(error),
    );
  }

  /// Sets the markets.
  Future<void> setMarkets(final List<FeedMarket> markets) async {
    final boxedMarkets = markets.allocVec();
    final boxedResult = Boxed(
      await asyncCore.setMarkets(_sharedEngine, boxedMarkets.move()),
      ffi.drop_result_void_string,
    );

    return resultVoidStringFfiAdapter.consumeNative(
      boxedResult,
      mapErr: (error) => Exception(error),
    );
  }

  /// Gets feed documents.
  Future<List<DocumentFfi>> getFeedDocuments(final int maxDocuments) async {
    final boxedResult = Boxed(
      await asyncCore.getFeedDocuments(_sharedEngine, maxDocuments),
      ffi.drop_result_vec_document_string,
    );

    return resultVecDocumentStringFfiAdapter.consumeNative(
      boxedResult,
      mapErr: (error) => Exception(error),
    );
  }

  /// Processes time spent.
  Future<void> timeSpent(
    final DocumentId id,
    final Float32List smbertEmbedding,
    final Duration time,
    final UserReaction reaction,
  ) async {
    final boxedTimeSpent = TimeSpentFfi(
      id: id,
      smbertEmbedding: smbertEmbedding,
      time: time,
      reaction: reaction,
    ).allocNative();
    final boxedResult = Boxed(
      await asyncCore.timeSpent(_sharedEngine, boxedTimeSpent.move()),
      ffi.drop_result_void_string,
    );

    return resultVoidStringFfiAdapter.consumeNative(
      boxedResult,
      mapErr: (error) => Exception(error),
    );
  }

  /// Processes user reaction.
  Future<void> userReacted(
    final DocumentId id,
    final StackId stackId,
    final String snippet,
    final Float32List smbertEmbedding,
    final UserReaction reaction,
  ) async {
    final boxedUserReacted = UserReactedFfi(
      id: id,
      stackId: stackId,
      snippet: snippet,
      smbertEmbedding: smbertEmbedding,
      reaction: reaction,
    ).allocNative();
    final boxedResult = Boxed(
      await asyncCore.userReacted(_sharedEngine, boxedUserReacted.move()),
      ffi.drop_result_void_string,
    );

    return resultVoidStringFfiAdapter.consumeNative(
      boxedResult,
      mapErr: (error) => Exception(error),
    );
  }

  /// Drops the engine.
  void free() {
    _sharedEngine = nullptr;
    _boxedResult.free();
  }
}
