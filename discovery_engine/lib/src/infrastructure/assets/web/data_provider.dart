import 'package:xayn_discovery_engine/src/domain/assets/asset.dart'
    show AssetType;
import 'package:xayn_discovery_engine/src/domain/assets/asset_fetcher.dart'
    show AssetFetcher;
import 'package:xayn_discovery_engine/src/domain/assets/data_provider.dart'
    show DataProvider, SetupData;
import 'package:xayn_discovery_engine/src/domain/assets/manifest_reader.dart'
    show ManifestReader;

const _baseAssetUrl = 'assets/assets';

class WebDataProvider extends DataProvider {
  @override
  final AssetFetcher assetFetcher;
  @override
  final ManifestReader manifestReader;
  @override
  // TODO: maybe we don't need it
  final Uri baseUri;

  WebDataProvider(
    this.assetFetcher,
    this.manifestReader,
    this.baseUri,
  );

  @override
  Future<SetupData> getSetupData() async {
    final fetched = <AssetType, dynamic>{};
    final manifest = await manifestReader.read();

    for (final asset in manifest.assets) {
      final path = DataProvider.joinPaths([_baseAssetUrl, asset.urlSuffix]);

      // We also load the wasm/worker script here in order to check its integrity/checksum.
      // The browser keeps it in cache so `injectWasmScript` does not download it again.
      final bytes = await assetFetcher.fetchAsset(asset);

      if (asset.id == AssetType.webWorkerScript ||
          asset.id == AssetType.wasmScript) {
        fetched.putIfAbsent(asset.id, () => path);
      } else {
        fetched.putIfAbsent(asset.id, () => bytes);
      }
    }

    return SetupData(fetched);
  }
}

DataProvider createDataProvider(
  final AssetFetcher assetFetcher,
  final ManifestReader manifestReader,
  final Uri baseUri,
) =>
    WebDataProvider(assetFetcher, manifestReader, baseUri);
