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
import 'package:xayn_discovery_engine/src/api/api.dart' show FeedMarket;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/feed_market.dart'
    show FeedMarketFfi;

void main() {
  test('reading written FeedMarked works', () {
    const market =
        FeedMarket(langCode: 'this is a string', countryCode: 'another string');
    final place = ffi.alloc_uninitialized_market();
    market.writeNative(place);
    final res = FeedMarketFfi.readNative(place);
    ffi.drop_market(place);
    expect(market, equals(res));
  });
}
