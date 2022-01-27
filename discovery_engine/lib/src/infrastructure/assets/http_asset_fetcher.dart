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
    show AssetFetcher, AssetFetcherException;
import 'package:xayn_discovery_engine/src/domain/assets/data_provider.dart'
    show DataProvider;
import 'package:xayn_discovery_engine/src/logger.dart' show logger;

class HttpAssetFetcher extends AssetFetcher {
  final String _baseUrl;

  HttpAssetFetcher(this._baseUrl);

  @override
  Future<Uint8List> fetchFragment(String urlSuffix) async {
    final url = DataProvider.joinPaths([_baseUrl, urlSuffix]);

    logger.i('AssetFetcher fetchFragment: $url');

    final client = RetryClient(
      http.Client(),
      retries: 2,
      onRetry: (req, res, retryCount) {
        final message =
            'AssetFetcher:\n  request url: ${req.url},\n  response statusCode: ${res?.statusCode},\n  retry nb: $retryCount';
        logger.i(message);
      },
    );

    final uri = Uri.tryParse(url);

    if (uri == null) {
      throw AssetFetcherException('Can\'t parse url: $url');
    }

    final response = await client
        .get(uri)
        .onError((error, stackTrace) => throw AssetFetcherException('$error'));

    if (response.statusCode != 200) {
      // triggers when the asset is not available on the provided url
      final message =
          'error loading asset: $uri,\n  status: ${response.statusCode}\n  error: ${response.reasonPhrase}';
      logger.e(message);
      throw AssetFetcherException(message);
    }

    return response.bodyBytes;
  }
}
