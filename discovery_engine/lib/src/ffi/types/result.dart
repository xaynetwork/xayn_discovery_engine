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

import 'dart:ffi' show NativeType, nullptr, Pointer;

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
  final Pointer<RustOk> Function(Pointer<RustResult>) moveOk;
  final Pointer<RustErr> Function(Pointer<RustResult>) moveErr;
  final Ok Function(Pointer<RustOk>) readNativeOk;
  final Err Function(Pointer<RustErr>) readNativeErr;
  final Never Function(Err) throwErr;
  final void Function(Pointer<RustOk>) freeOk;
  final void Function(Pointer<RustErr>) freeErr;
  final void Function(Pointer<RustResult>) freeResult;

  ResultFfiAdapter({
    required this.getOk,
    required this.getErr,
    required this.moveOk,
    required this.moveErr,
    required this.readNativeOk,
    required this.readNativeErr,
    required this.throwErr,
    required this.freeOk,
    required this.freeErr,
    required this.freeResult,
  });

  /// Reads the result and returns the success value or throws in case of an error value.
  Ok readNative(Pointer<RustResult> result) {
    final ok = getOk(result);
    if (ok != nullptr) {
      return readNativeOk(ok);
    }
    final err = getErr(result);
    if (err != nullptr) {
      throwErr(readNativeErr(err));
    }
    throw AssertionError('result should be either Ok or Err');
  }

  /// Consumes the result and returns the success value or throws in case of an error value.
  ///
  /// # Safety
  /// Must only be called on owned pointers.
  Ok consumeNative(Pointer<RustResult> result) {
    try {
      return readNative(result);
    } finally {
      freeResult(result);
    }
  }

  /// Consumes the result and moves the success value or throws in case of an error value.
  ///
  /// # Safety
  /// Must only be called on owned pointers.
  Boxed<RustOk> moveNative(Pointer<RustResult> result) {
    try {
      readNative(result);
    } catch (_) {
      freeResult(result);
      rethrow;
    }

    return Boxed(moveOk(result), freeOk);
  }
}

Never _moveUnsupported<RustType extends NativeType>(Pointer<RustType> _) {
  throw UnsupportedError('get the value or consume the whole result');
}

Never _throwStringErr(final String error) {
  throw Exception(error);
}

final resultVoidStringFfiAdapter = ResultFfiAdapter(
  getOk: ffi.get_result_void_string_ok,
  getErr: ffi.get_result_void_string_err,
  moveOk: _moveUnsupported,
  moveErr: _moveUnsupported,
  readNativeOk: (_) {},
  readNativeErr: StringFfi.readNative,
  throwErr: _throwStringErr,
  freeOk: _moveUnsupported,
  freeErr: _moveUnsupported,
  freeResult: ffi.drop_result_void_string,
);

final resultVecU8StringFfiAdapter = ResultFfiAdapter(
  getOk: ffi.get_result_vec_u8_string_ok,
  getErr: ffi.get_result_vec_u8_string_err,
  moveOk: _moveUnsupported,
  moveErr: _moveUnsupported,
  readNativeOk: Uint8ListFfi.readNative,
  readNativeErr: StringFfi.readNative,
  throwErr: _throwStringErr,
  freeOk: _moveUnsupported,
  freeErr: _moveUnsupported,
  freeResult: ffi.drop_result_vec_u8_string,
);

final resultVecDocumentStringFfiAdapter = ResultFfiAdapter(
  getOk: ffi.get_result_vec_document_string_ok,
  getErr: ffi.get_result_vec_document_string_err,
  moveOk: _moveUnsupported,
  moveErr: _moveUnsupported,
  readNativeOk: DocumentSliceFfi.readVec,
  readNativeErr: StringFfi.readNative,
  throwErr: _throwStringErr,
  freeOk: _moveUnsupported,
  freeErr: _moveUnsupported,
  freeResult: ffi.drop_result_vec_document_string,
);

final resultSharedEngineStringFfiAdapter = ResultFfiAdapter(
  getOk: ffi.get_result_shared_engine_string_ok,
  getErr: ffi.get_result_shared_engine_string_err,
  moveOk: ffi.move_result_shared_engine_string_ok,
  moveErr: ffi.move_result_shared_engine_string_err,
  readNativeOk: (final Pointer<RustSharedEngine> sharedEngine) => sharedEngine,
  readNativeErr: StringFfi.readNative,
  throwErr: _throwStringErr,
  freeOk: ffi.drop_shared_engine,
  freeErr: ffi.drop_string,
  freeResult: ffi.drop_result_shared_engine_string,
);
