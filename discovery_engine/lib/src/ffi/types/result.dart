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

abstract class ResultFfiAdapter<Ok, Err, RustResult extends NativeType,
    RustOk extends NativeType, RustErr extends NativeType> {
  late final Pointer<RustOk> Function(Pointer<RustResult>) _getOk;
  late final Pointer<RustErr> Function(Pointer<RustResult>) _getErr;
  late final Ok Function(Pointer<RustOk>) _readNativeOk;
  late final Err Function(Pointer<RustErr>) _readNativeErr;
  late final Never Function(Err) _throwErr;
  late final void Function(Pointer<RustResult>) _freeResult;

  /// Reads the result and returns the success value or throws in case of an error value.
  Ok readNative(Pointer<RustResult> result) {
    final ok = _getOk(result);
    if (ok != nullptr) {
      return _readNativeOk(ok);
    }
    final err = _getErr(result);
    if (err != nullptr) {
      _throwErr(_readNativeErr(err));
    }
    throw AssertionError('result should be either Ok or Err');
  }

  /// Frees the result.
  ///
  /// # Safety
  /// Must only be called on owned pointers and not more than once.
  void freeNative(Pointer<RustResult> result) {
    _freeResult(result);
  }
}

mixin ConsumeResultFfiMixin<Ok, Err, RustResult extends NativeType,
        RustOk extends NativeType, RustErr extends NativeType>
    on ResultFfiAdapter<Ok, Err, RustResult, RustOk, RustErr> {
  /// Consumes the result and returns the success value or throws in case of an error value.
  ///
  /// # Safety
  ///
  /// Must only be called on owned pointers and not more than once.
  Ok consumeNative(Pointer<RustResult> result) {
    try {
      return readNative(result);
    } finally {
      freeNative(result);
    }
  }
}

mixin MoveResultFfiMixin<Ok, Err, RustResult extends NativeType,
        RustOk extends NativeType, RustErr extends NativeType>
    on ResultFfiAdapter<Ok, Err, RustResult, RustOk, RustErr> {
  late final Pointer<RustOk> Function(Pointer<RustResult>) _moveOk;
  late final void Function(Pointer<RustOk>) _freeOk;

  /// Consumes the result and moves the success value or throws in case of an error value.
  ///
  /// # Safety
  ///
  /// Must only be called on owned pointers and not more than once.
  Boxed<RustOk> moveNative(Pointer<RustResult> result) {
    try {
      readNative(result);
    } catch (_) {
      freeNative(result);
      rethrow;
    }

    return Boxed(_moveOk(result), _freeOk);
  }
}

class ConsumeResultFfiAdapter<Ok, Err, RustResult extends NativeType,
        RustOk extends NativeType, RustErr extends NativeType>
    extends ResultFfiAdapter<Ok, Err, RustResult, RustOk, RustErr>
    with ConsumeResultFfiMixin<Ok, Err, RustResult, RustOk, RustErr> {
  ConsumeResultFfiAdapter({
    required final Pointer<RustOk> Function(Pointer<RustResult>) getOk,
    required final Pointer<RustErr> Function(Pointer<RustResult>) getErr,
    required final Ok Function(Pointer<RustOk>) readNativeOk,
    required final Err Function(Pointer<RustErr>) readNativeErr,
    required final Never Function(Err) throwErr,
    required final void Function(Pointer<RustResult>) freeResult,
  }) {
    _getOk = getOk;
    _getErr = getErr;
    _readNativeOk = readNativeOk;
    _readNativeErr = readNativeErr;
    _throwErr = throwErr;
    _freeResult = freeResult;
  }
}

class MoveResultFfiAdapter<Ok, Err, RustResult extends NativeType,
        RustOk extends NativeType, RustErr extends NativeType>
    extends ResultFfiAdapter<Ok, Err, RustResult, RustOk, RustErr>
    with MoveResultFfiMixin<Ok, Err, RustResult, RustOk, RustErr> {
  MoveResultFfiAdapter({
    required final Pointer<RustOk> Function(Pointer<RustResult>) getOk,
    required final Pointer<RustErr> Function(Pointer<RustResult>) getErr,
    required final Pointer<RustOk> Function(Pointer<RustResult>) moveOk,
    required final Ok Function(Pointer<RustOk>) readNativeOk,
    required final Err Function(Pointer<RustErr>) readNativeErr,
    required final Never Function(Err) throwErr,
    required final void Function(Pointer<RustOk>) freeOk,
    required final void Function(Pointer<RustResult>) freeResult,
  }) {
    _getOk = getOk;
    _getErr = getErr;
    _moveOk = moveOk;
    _readNativeOk = readNativeOk;
    _readNativeErr = readNativeErr;
    _throwErr = throwErr;
    _freeOk = freeOk;
    _freeResult = freeResult;
  }
}

Never _throwStringErr(final String error) {
  throw Exception(error);
}

final resultVoidStringFfiAdapter = ConsumeResultFfiAdapter(
  getOk: ffi.get_result_void_string_ok,
  getErr: ffi.get_result_void_string_err,
  readNativeOk: (_) {},
  readNativeErr: StringFfi.readNative,
  throwErr: _throwStringErr,
  freeResult: ffi.drop_result_void_string,
);

final resultVecU8StringFfiAdapter = ConsumeResultFfiAdapter(
  getOk: ffi.get_result_vec_u8_string_ok,
  getErr: ffi.get_result_vec_u8_string_err,
  readNativeOk: Uint8ListFfi.readNative,
  readNativeErr: StringFfi.readNative,
  throwErr: _throwStringErr,
  freeResult: ffi.drop_result_vec_u8_string,
);

final resultVecDocumentStringFfiAdapter = ConsumeResultFfiAdapter(
  getOk: ffi.get_result_vec_document_string_ok,
  getErr: ffi.get_result_vec_document_string_err,
  readNativeOk: DocumentSliceFfi.readVec,
  readNativeErr: StringFfi.readNative,
  throwErr: _throwStringErr,
  freeResult: ffi.drop_result_vec_document_string,
);

final resultSharedEngineStringFfiAdapter = MoveResultFfiAdapter(
  getOk: ffi.get_result_shared_engine_string_ok,
  getErr: ffi.get_result_shared_engine_string_err,
  moveOk: ffi.move_result_shared_engine_string_ok,
  readNativeOk: (final Pointer<RustSharedEngine> sharedEngine) => sharedEngine,
  readNativeErr: StringFfi.readNative,
  throwErr: _throwStringErr,
  freeOk: ffi.drop_shared_engine,
  freeResult: ffi.drop_result_shared_engine_string,
);
