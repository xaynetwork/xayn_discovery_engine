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
  Future<Uint8List> fetch(String urlSuffix) async {
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
