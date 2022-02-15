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
import 'dart:typed_data' show Float32List;

import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/data_provider.dart'
    show NativeSetupData;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustResultEngine, RustEngine, RustVecU8;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart'
    show asyncCore, ffi;
import 'package:xayn_discovery_engine/src/ffi/types/document/document.dart'
    show DocumentFfi;
import 'package:xayn_discovery_engine/src/ffi/types/document/document_vec.dart'
    show DocumentSliceFfi;
import 'package:xayn_discovery_engine/src/ffi/types/document/time_spent.dart'
    show TimeSpentFfi;
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reacted.dart'
    show UserReactedFfi;
import 'package:xayn_discovery_engine/src/ffi/types/init_config.dart'
    show InitConfigFfi;

class BoxedEngine {
  final Pointer<RustEngine> _ptr;

  BoxedEngine._(this._ptr);

  Future<BoxedEngine> initialize(
    Configuration config,
    NativeSetupData setupData,
    // TODO: add Uint8List handling to ListFfiAdapter
    List<int>? state,
  ) async {
    final configPtr = ffi.alloc_uninitialized_init_config();
    InitConfigFfi(config, setupData).writeNative(configPtr);

    final statePtr = nullptr;
    if (state != null) {
      // TODO: impl alloc_uninit for RustVecU8 and impl ListFfiAdapter
      statePtr = ffi.alloc_uninitialized_bytes_vec(state.length);
      state.writeVec(statePtr);
    }

    final result = await asyncCore.initialize(configPlace, statePlace);
    // TODO: impl RustResultEngine getters
    final engine = ffi.get_result_engine_ok(result);
    if (engine == null) {
      // TODO: free RustString error
      throw Exception('${ffi.get_result_engine_err(result)}');
    }

    return BoxedEngine._(engine);
  }

  Future<List<int>> serialize() async {
    final result = await asyncCore.serialize(_ptr);
    // TODO: impl RustResultVecU8 getters
    final bytes = ffi.get_result_bytes_vec_ok(result);
    if (bytes == null) {
      // TODO: free RustString error
      throw Exception('${ffi.get_result_bytes_vec_err(result)}');
    }

    // TODO: impl ByteSliceFfi for List<int>/Uint8List
    return ByteSliceFfi.consumeBoxedVector(bytes);
  }

  Future<void> setMarkets(List<FeedMarket> markets) async {
    // TODO: impl alloc_uninit for RustVecMarket and impl ListFfiAdapter
    final marketsPtr = ffi.alloc_uninitialized_market_vec(markets.length);
    markets.writeVec(marketsPtr);

    final result = await asyncCore.setMarkets(_ptr, marketsPtr);
    // TODO: impl RustResultVoid getters
    final error = ffi.get_result_void_err(result);
    if (error != null) {
      // TODO: free RustString error
      throw Exception('$error');
    }

    return;
  }

  Future<List<DocumentFfi>> getFeedDocuments(int maxDocuments) async {
    final result = await asyncCore.getFeedDocuments(_ptr, maxDocuments);
    // TODO: impl RustResultVecDocument getters
    final documents = ffi.get_result_document_vec_ok(result);
    if (documents == null) {
      // TODO: free RustString error
      throw Exception('${ffi.get_result_document_vec_err(result)}');
    }

    return DocumentSliceFfi.consumeBoxedVector(documents);
  }

  Future<void> timeSpent(
    DocumentId id,
    Float32List smbertEmbedding,
    Duration time,
    UserReaction reaction,
  ) async {
    final timeSpentPtr = ffi.alloc_uninitialized_time_spend();
    TimeSpentFfi(
      id: id,
      smbertEmbedding: smbertEmbedding,
      time: time,
      reaction: reaction,
    ).writeTo(timeSpentPtr);

    final result = await asyncCore.timeSpent(_ptr, timeSpentPtr);
    // TODO: impl RustResultVoid getters
    final error = ffi.get_result_void_err(result);
    if (error != null) {
      // TODO: free RustString error
      throw Exception('$error');
    }

    return;
  }

  Future<void> userReacted(
    DocumentId id,
    StackId stackId,
    String snippet,
    Float32List smbertEmbedding,
    UserReaction reaction,
  ) async {
    final userReactedPtr = ffi.alloc_uninitialized_user_reacted();
    UserReactedFfi(
      id: id,
      stackId: stackId,
      snippet: snippet,
      smbertEmbedding: smbertEmbedding,
      reaction: reaction,
    ).writeTo(userReactedPtr);

    final result = await asyncCore.userReacted(_ptr, userReactedPtr);
    // TODO: impl RustResultVoid getters
    final error = ffi.get_result_void_err(result);
    if (error != null) {
      // TODO: free RustString error
      throw Exception('$error');
    }

    return;
  }
}
