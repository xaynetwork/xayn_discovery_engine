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
    show RustInitializationResult;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show asyncFfi, ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/document/document.dart';
import 'package:xayn_discovery_engine/src/ffi/types/document/document_vec.dart'
    show DocumentSliceFfi;
import 'package:xayn_discovery_engine/src/ffi/types/search.dart' show SearchFfi;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart'
    show StringFfi, StringListFfi;

abstract class ResultFfiAdapter<Ok, Err, RustResult extends NativeType,
    RustOk extends NativeType, RustErr extends NativeType> {
  final Pointer<RustOk> Function(Pointer<RustResult>) _getOk;
  final Pointer<RustErr> Function(Pointer<RustResult>) _getErr;
  final Ok Function(Pointer<RustOk>) _readNativeOk;
  final Err Function(Pointer<RustErr>) _readNativeErr;
  final Never Function(Err) _throwErr;
  final void Function(Pointer<RustResult>) _freeResult;

  ResultFfiAdapter({
    required final Pointer<RustOk> Function(Pointer<RustResult>) getOk,
    required final Pointer<RustErr> Function(Pointer<RustResult>) getErr,
    required final Ok Function(Pointer<RustOk>) readNativeOk,
    required final Err Function(Pointer<RustErr>) readNativeErr,
    required final Never Function(Err) throwErr,
    required final void Function(Pointer<RustResult>) freeResult,
  })  : _getOk = getOk,
        _getErr = getErr,
        _readNativeOk = readNativeOk,
        _readNativeErr = readNativeErr,
        _throwErr = throwErr,
        _freeResult = freeResult;

  /// Reads the result and returns the success value or throws in case of an error value.
  Ok readNative(final Pointer<RustResult> result) {
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
  void freeNative(final Pointer<RustResult> result) {
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
  Ok consumeNative(final Pointer<RustResult> result) {
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
  Pointer<RustOk> Function(Pointer<RustResult>) get _moveOk;
  void Function(Pointer<RustOk>) get _freeOk;

  /// Consumes the result and moves the success value or throws in case of an error value.
  ///
  /// # Safety
  ///
  /// Must only be called on owned pointers and not more than once.
  Boxed<RustOk> moveNative(final Pointer<RustResult> result) {
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
  }) : super(
          getOk: getOk,
          getErr: getErr,
          readNativeOk: readNativeOk,
          readNativeErr: readNativeErr,
          throwErr: throwErr,
          freeResult: freeResult,
        );
}

class MoveResultFfiAdapter<Ok, Err, RustResult extends NativeType,
        RustOk extends NativeType, RustErr extends NativeType>
    extends ResultFfiAdapter<Ok, Err, RustResult, RustOk, RustErr>
    with MoveResultFfiMixin<Ok, Err, RustResult, RustOk, RustErr> {
  @override
  final Pointer<RustOk> Function(Pointer<RustResult>) _moveOk;
  @override
  final void Function(Pointer<RustOk>) _freeOk;

  MoveResultFfiAdapter({
    required final Pointer<RustOk> Function(Pointer<RustResult>) getOk,
    required final Pointer<RustErr> Function(Pointer<RustResult>) getErr,
    required final Pointer<RustOk> Function(Pointer<RustResult>) moveOk,
    required final Ok Function(Pointer<RustOk>) readNativeOk,
    required final Err Function(Pointer<RustErr>) readNativeErr,
    required final Never Function(Err) throwErr,
    required final void Function(Pointer<RustOk>) freeOk,
    required final void Function(Pointer<RustResult>) freeResult,
  })  : _moveOk = moveOk,
        _freeOk = freeOk,
        super(
          getOk: getOk,
          getErr: getErr,
          readNativeOk: readNativeOk,
          readNativeErr: readNativeErr,
          throwErr: throwErr,
          freeResult: freeResult,
        );
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

final resultDocumentStringFfiAdapter = ConsumeResultFfiAdapter(
  getOk: ffi.get_result_document_string_ok,
  getErr: ffi.get_result_document_string_err,
  readNativeOk: DocumentFfi.readNative,
  readNativeErr: StringFfi.readNative,
  throwErr: _throwStringErr,
  freeResult: ffi.drop_result_document_string,
);

final resultVecDocumentStringFfiAdapter = ConsumeResultFfiAdapter(
  getOk: ffi.get_result_vec_document_string_ok,
  getErr: ffi.get_result_vec_document_string_err,
  readNativeOk: DocumentSliceFfi.readVec,
  readNativeErr: StringFfi.readNative,
  throwErr: _throwStringErr,
  freeResult: ffi.drop_result_vec_document_string,
);

final resultInitializationResultStringFfiAdapter = MoveResultFfiAdapter(
  getOk: ffi.get_result_initialization_result_string_ok,
  getErr: ffi.get_result_initialization_result_string_err,
  moveOk: ffi.move_result_initialization_result_string_ok,
  readNativeOk: (final Pointer<RustInitializationResult> initResult) =>
      initResult,
  readNativeErr: StringFfi.readNative,
  throwErr: _throwStringErr,
  freeOk: (final Pointer<RustInitializationResult> initResult) {
    final sharedEngine =
        ffi.destruct_initialization_result_into_shared_engine(initResult);
    asyncFfi.dispose(sharedEngine);
  },
  freeResult: ffi.drop_result_initialization_result_string,
);

final resultSearchStringFfiAdapter = ConsumeResultFfiAdapter(
  getOk: ffi.get_result_search_string_ok,
  getErr: ffi.get_result_search_string_err,
  readNativeOk: SearchFfi.readNative,
  readNativeErr: StringFfi.readNative,
  throwErr: _throwStringErr,
  freeResult: ffi.drop_result_search_string,
);

final resultVecStringStringFfiAdapter = ConsumeResultFfiAdapter(
  getOk: ffi.get_result_vec_string_string_ok,
  getErr: ffi.get_result_vec_string_string_err,
  readNativeOk: StringListFfi.readNative,
  readNativeErr: StringFfi.readNative,
  throwErr: _throwStringErr,
  freeResult: ffi.drop_result_vec_string_string,
);
