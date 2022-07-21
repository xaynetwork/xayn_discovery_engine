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

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    show ActiveSearch, SearchBy;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/search.dart'
    show SearchByFfi, SearchFfi;

void main() {
  test('reading written search-by yields same result', () {
    for (final searchBy in [SearchBy.query, SearchBy.topic]) {
      final place = ffi.alloc_uninitialized_search_by();
      searchBy.writeNative(place);
      final res = SearchByFfi.readNative(place);
      ffi.drop_search_by(place);
      expect(res, equals(searchBy));
    }
  });

  test('reading written search instance yields same result', () {
    final search = ActiveSearch(
      searchBy: SearchBy.query,
      searchTerm: 'example',
      requestedPageNb: -1,
      pageSize: -1,
    );
    final boxed = search.allocNative();
    final res = SearchFfi.readNative(boxed.ref);
    boxed.free();
    expect(res, equals(search));
  });
}
