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

import 'dart:ffi';

import 'package:equatable/equatable.dart';
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart';
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart';
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart';
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show StringFfi;

class WeightedSourceFfi with EquatableMixin {
  final String source;
  final int weight;

  WeightedSourceFfi({
    required this.source,
    required this.weight,
  });

  @override
  List<Object?> get props => [source, weight];

  factory WeightedSourceFfi.readNative(
    final Pointer<RustWeightedSource> place,
  ) {
    return WeightedSourceFfi(
      source: StringFfi.readNative(ffi.weighted_source_place_of_source(place)),
      weight: ffi.weighted_source_place_of_weight(place).value,
    );
  }

  WeightedSourceFfi.fromSourceReacted(SourceReacted source)
      : source = source.source.value,
        weight = source.weight;

  void writeNative(final Pointer<RustWeightedSource> place) {
    source.writeNative(ffi.weighted_source_place_of_source(place));
    ffi.weighted_source_place_of_weight(place).value = weight;
  }

  Boxed<RustWeightedSource> allocNative() {
    final place = ffi.alloc_uninitialized_weighted_source();
    writeNative(place);
    return Boxed(place, ffi.drop_weighted_source);
  }
}
