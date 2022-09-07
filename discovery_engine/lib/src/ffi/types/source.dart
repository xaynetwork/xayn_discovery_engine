import 'dart:ffi';

import 'package:xayn_discovery_engine/src/domain/models/source.dart';
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart';
import 'package:xayn_discovery_engine/src/ffi/types/string.dart';

extension SetSourceFfi on Set<Source> {
  /// Writes a `Vec<String>` to given place.
  void writeNative(
    final Pointer<RustVecString> place,
  ) =>
      listAdapter.writeVec(map((source) => source.toString()).toList(), place);
}
