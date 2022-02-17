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

import 'dart:ffi' show NativeType, Pointer;

import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustSharedEngine;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/document/document_vec.dart'
    show DocumentSliceFfi;
import 'package:xayn_discovery_engine/src/ffi/types/primitives.dart'
    show Uint8ListFfi;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show StringFfi;

class ResultFfiAdapter<Ok, Err, RustResult extends NativeType,
    RustOk extends NativeType, RustErr extends NativeType> {
  final Pointer<RustOk> Function(Pointer<RustResult>) getOk;
  final Pointer<RustErr> Function(Pointer<RustResult>) getErr;
  final Ok Function(Pointer<RustOk>) readNativeOk;
  final Err Function(Pointer<RustErr>) readNativeErr;

  ResultFfiAdapter({
    required this.getOk,
    required this.getErr,
    required this.readNativeOk,
    required this.readNativeErr,
  });

  Ok readNative(
    Pointer<RustResult> result, {
    required Exception Function(Err) mapErr,
  }) {
    final ok = getOk(result);
    if (ok.address != 0) {
      return readNativeOk(ok);
    }
    final err = getErr(result);
    if (err.address != 0) {
      throw mapErr(readNativeErr(err));
    }
    throw AssertionError('result should be either Ok or Err');
  }

  Ok consumeNative(
    Boxed<RustResult> result, {
    required Exception Function(Err) mapErr,
  }) {
    try {
      return readNative(result.ref, mapErr: mapErr);
    } finally {
      result.free();
    }
  }
}

final resultVoidStringFfiAdapter = ResultFfiAdapter(
  getOk: ffi.get_result_void_string_ok,
  getErr: ffi.get_result_void_string_err,
  readNativeOk: (_) {},
  readNativeErr: StringFfi.readNative,
);

final resultVecU8StringFfiAdapter = ResultFfiAdapter(
  getOk: ffi.get_result_vec_u8_string_ok,
  getErr: ffi.get_result_vec_u8_string_err,
  readNativeOk: Uint8ListFfi.readNative,
  readNativeErr: StringFfi.readNative,
);

final resultVecDocumentStringFfiAdapter = ResultFfiAdapter(
  getOk: ffi.get_result_vec_document_string_ok,
  getErr: ffi.get_result_vec_document_string_err,
  readNativeOk: DocumentSliceFfi.readVec,
  readNativeErr: StringFfi.readNative,
);

final resultSharedEngineStringFfiAdapter = ResultFfiAdapter(
  getOk: ffi.get_result_shared_engine_string_ok,
  getErr: ffi.get_result_shared_engine_string_err,
  readNativeOk: (final Pointer<RustSharedEngine> sharedEngine) => sharedEngine,
  readNativeErr: StringFfi.readNative,
);
