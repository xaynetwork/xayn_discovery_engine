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

import 'package:xayn_discovery_engine/src/domain/models/active_search.dart';
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart';
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart';
import 'package:xayn_discovery_engine/src/ffi/types/primitives.dart';
import 'package:xayn_discovery_engine/src/ffi/types/search.dart'
    show SearchByFfi;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart';

extension ActiveSearchFfi on ActiveSearch {
  Boxed<RustMigrationSearch> allocNative() {
    final place = ffi.alloc_uninitialized_migration_search();
    writeNative(place);
    return Boxed(place, ffi.drop_search);
  }

  void writeNative(Pointer<RustMigrationSearch> place) {
    searchBy.writeNative(ffi.migration_search_place_of_search_by(place));
    searchTerm.writeNative(ffi.migration_search_place_of_search_term(place));
    pageSize.writeNative(ffi.migration_search_place_of_page_size(place));
    requestedPageNb.writeNative(ffi.migration_search_place_of_next_page(place));
  }
}

extension OptionActiveSearchFfi on ActiveSearch? {
  void writeNative(Pointer<RustOptionMigrationSearch> place) {
    final self = this;
    if (self == null) {
      ffi.init_option_migration_search_none_at(place);
    } else {
      final search = self.allocNative();
      ffi.init_option_migration_search_some_at(place, search.move());
    }
  }
}
