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

import 'package:http/http.dart' as http;
import 'package:http_retry/http_retry.dart' show RetryClient;
import 'package:xayn_discovery_engine/src/domain/assets/asset_fetcher.dart'
    show AssetFetcher;
import 'package:xayn_discovery_engine/src/domain/assets/data_provider.dart'
    show DataProvider;

class HttpAssetFetcher extends AssetFetcher {
  final String _baseUrl;

  HttpAssetFetcher(this._baseUrl);

  @override
  Future<Uint8List> fetchFragment(String urlSuffix) async {
    // TODO: maybe configure RetryClient
    final client = RetryClient(http.Client());

    final url = DataProvider.joinPaths([_baseUrl, urlSuffix]);
    final uri = Uri.parse(url);
    final response = await client.get(uri);

    if (response.statusCode != 200) {
      // triggers when the asset is not available on the provided url
      final msg =
          'error loading asset: $uri,\nstatus: ${response.statusCode}\nerror: ${response.reasonPhrase}';
      return Future.error(msg);
    }

    return response.bodyBytes;
  }
}
