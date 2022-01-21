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

import 'dart:typed_data' show Uint8List;

import 'package:xayn_discovery_engine/src/infrastructure/assets/http_asset_fetcher.dart'
    show HttpAssetFetcher;

class HttpAssetFetcherWithCounter extends HttpAssetFetcher {
  int _callCount = 0;
  int get callCount => _callCount;

  HttpAssetFetcherWithCounter(String baseUrl) : super(baseUrl);

  @override
  Future<Uint8List> fetchFragment(String urlSuffix) async {
    _callCount += 1;
    return super.fetchFragment(urlSuffix);
  }

  void resetCount() {
    _callCount = 0;
  }
}
