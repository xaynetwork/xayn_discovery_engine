import 'package:xayn_discovery_engine/src/domain/assets/asset.dart'
    show AssetType;
import 'package:xayn_discovery_engine/src/domain/assets/asset_fetcher.dart'
    show AssetFetcher;
import 'package:xayn_discovery_engine/src/domain/assets/manifest_reader.dart'
    show ManifestReader;

/// Data that is required to initialize [`XaynAi`].
class SetupData {
  SetupData(Map<AssetType, dynamic> assets) {
    throw UnsupportedError('Unsupported platform.');
  }
}

/// Reads the assets manifest and provides the [SetupData] to further use.
abstract class DataProvider {
  AssetFetcher get assetFetcher;
  ManifestReader get manifestReader;
  Uri get baseUri;

  Future<SetupData> getSetupData() {
    throw UnsupportedError('Unsupported platform.');
  }

  static String joinPaths(List<String> paths) {
    return paths.where((e) => e.isNotEmpty).join('/');
  }
}
