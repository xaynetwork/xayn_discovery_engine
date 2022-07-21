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

import 'dart:ffi' show Pointer, Uint8Pointer;

import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    show ActiveSearch, SearchBy, SearchByIntConversion;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustSearch, RustSearchBy1;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show StringFfi;

extension SearchByFfi on SearchBy {
  void writeNative(final Pointer<RustSearchBy1> place) {
    place.value = toIntRepr();
  }

  static SearchBy readNative(
    final Pointer<RustSearchBy1> place,
  ) =>
      SearchByIntConversion.fromIntRepr(place.value);
}

extension SearchFfi on ActiveSearch {
  static ActiveSearch readNative(final Pointer<RustSearch> place) {
    return ActiveSearch(
      searchBy: SearchByFfi.readNative(ffi.search_place_of_by(place)),
      searchTerm: StringFfi.readNative(ffi.search_place_of_term(place)),

      // TODO: remove once DB is migrated (unused but present because of legacy reasons)
      requestedPageNb: -1,
      pageSize: -1,
    );
  }

  Boxed<RustSearch> allocNative() {
    final place = ffi.alloc_uninitialized_search();
    writeNative(place);
    return Boxed(place, ffi.drop_search);
  }

  void writeNative(final Pointer<RustSearch> place) {
    searchBy.writeNative(ffi.search_place_of_by(place));
    searchTerm.writeNative(ffi.search_place_of_term(place));
  }
}
