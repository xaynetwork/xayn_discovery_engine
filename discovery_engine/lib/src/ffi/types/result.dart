import 'dart:ffi' show NativeType, Pointer;

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
