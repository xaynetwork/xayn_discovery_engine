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

import 'dart:ffi' show Pointer;

import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustMarket;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show StringFfi;

extension FeedMarketFfi on FeedMarket {
  void writeNative(Pointer<RustMarket> place) {
    countryCode.writeNative(ffi.market_place_of_country_code(place));
    langCode.writeNative(ffi.market_place_of_lang_code(place));
    ffi.finish_market_initialization(place);
  }

  static FeedMarket readNative(Pointer<RustMarket> market) {
    return FeedMarket(
      countryCode:
          StringFfi.readNative(ffi.market_place_of_country_code(market)),
      langCode: StringFfi.readNative(ffi.market_place_of_lang_code(market)),
    );
  }
}
