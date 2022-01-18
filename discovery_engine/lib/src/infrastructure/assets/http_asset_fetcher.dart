import 'dart:typed_data' show Uint8List;

import 'package:http/http.dart' as http;
import 'package:xayn_discovery_engine/src/domain/assets/asset_fetcher.dart'
    show AssetFetcher;
import 'package:xayn_discovery_engine/src/domain/assets/data_provider.dart'
    show DataProvider;

class HttpAssetFetcher extends AssetFetcher {
  final String _baseUrl;
  final http.Client _client;

  HttpAssetFetcher(this._baseUrl) : _client = http.Client();

  @override
  Future<Uint8List> fetch(String urlSuffix) async {
    final url = DataProvider.joinPaths([_baseUrl, urlSuffix]);
    final uri = Uri.parse(url);
    final response = await _client.get(uri);

    if (response.statusCode != 200) {
      // triggers when the asset is not available on the provided url
      final msg =
          'error loading asset: $uri,\nstatus: ${response.statusCode}\nerror: ${response.reasonPhrase}';
      return Future.error(msg);
    }

    return response.bodyBytes;
  }
}
