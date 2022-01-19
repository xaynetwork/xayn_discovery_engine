import 'dart:typed_data' show Uint8List;

import 'package:xayn_discovery_engine/src/infrastructure/assets/http_asset_fetcher.dart'
    show HttpAssetFetcher;

class MockHttpAssetFetcher extends HttpAssetFetcher {
  int _callCount = 0;
  int get callCount => _callCount;

  MockHttpAssetFetcher(String baseUrl) : super(baseUrl);

  @override
  Future<Uint8List> fetchFragment(String urlSuffix) async {
    _callCount += 1;
    return super.fetchFragment(urlSuffix);
  }
}
