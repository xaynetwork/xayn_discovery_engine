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
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/ffi/types/feed_market_vec.dart'
    show FeedMarketSliceFfi;

void main() {
  test('reading written FeedMarked works', () {
    const market = [
      FeedMarket(langCode: 'this is a string', countryCode: 'another string'),
      FeedMarket(langCode: 'de', countryCode: 'DE')
    ];
    final boxed = market.allocVec();
    final res = FeedMarketSliceFfi.readVec(boxed.ref);
    boxed.free();
    expect(market, equals(res));
  });
}
