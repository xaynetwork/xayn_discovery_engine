import 'dart:io' show File;
import 'dart:typed_data' show Uint8List;

import 'package:xayn_discovery_engine/src/domain/assets/asset_fetcher.dart'
    show AssetFetcher;
import 'package:xayn_discovery_engine/src/domain/assets/data_provider.dart'
    show DataProvider;

class FileAssetFetcher extends AssetFetcher {
  final String baseDirectoryPath;

  FileAssetFetcher(this.baseDirectoryPath);

  @override
  Future<Uint8List> fetch(String urlSuffix) async {
    final path = DataProvider.joinPaths([baseDirectoryPath, urlSuffix]);
    final file = File(path);

    if (!file.existsSync()) {
      final msg = 'Unable to fetch static AI files:\nurl: $path';
      return Future.error(msg);
    }

    return File(path).readAsBytes();
  }
}
