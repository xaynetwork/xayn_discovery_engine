import 'dart:io' show File, Directory;
import 'package:crypto/crypto.dart' show sha256;
import 'package:xayn_discovery_engine/src/domain/assets/asset.dart'
    show Asset, AssetType;
import 'package:xayn_discovery_engine/src/domain/assets/asset_fetcher.dart'
    show AssetFetcher;
import 'package:xayn_discovery_engine/src/domain/assets/data_provider.dart'
    show DataProvider, SetupData;
import 'package:xayn_discovery_engine/src/domain/assets/manifest_reader.dart'
    show ManifestReader;

const _baseAssetsPath = 'assets';

class NativeDataProvider extends DataProvider {
  @override
  final AssetFetcher assetFetcher;
  @override
  final ManifestReader manifestReader;
  @override
  final Uri baseUri;

  NativeDataProvider(
    this.assetFetcher,
    this.manifestReader,
    this.baseUri,
  );

  String get baseDirectoryPath =>
      DataProvider.joinPaths([baseUri.toFilePath(), _baseAssetsPath]);

  @override
  Future<SetupData> getSetupData() async {
    final paths = <AssetType, String>{};

    final manifest = await manifestReader.read();

    for (final asset in manifest.assets) {
      final path = await _getData(asset);
      paths.putIfAbsent(asset.id, () => path);
    }

    return SetupData(paths);
  }

  /// Returns the path to the data, if the data is not on disk yet
  /// it will be copied from the bundle to the disk.
  Future<String> _getData(Asset asset) async {
    final filePath =
        DataProvider.joinPaths([baseDirectoryPath, asset.urlSuffix]);
    final assetFile = File(filePath);
    final diskDirPath = assetFile.parent.path;
    await Directory(diskDirPath).create(recursive: true);

    // Only write the data on disk if the file does not exist or the checksum does not match.
    // The last check is useful in case the app is closed before we can finish to write,
    // and it can be also useful during development to test with different models.
    if (!assetFile.existsSync() ||
        !await _verifyChecksum(assetFile, asset.checksum.checksumAsHex)) {
      await assetFile.delete();
      final bytes = await assetFetcher.fetchAsset(asset);
      await assetFile.writeAsBytes(bytes, flush: true);
    }

    return assetFile.path;
  }

  Future<bool> _verifyChecksum(File file, String checksum) async {
    final digest = await sha256.bind(file.openRead()).first;
    return digest.toString() == checksum;
  }
}

DataProvider createDataProvider(
  final AssetFetcher assetFetcher,
  final ManifestReader manifestReader,
  final Uri baseUri,
) =>
    NativeDataProvider(assetFetcher, manifestReader, baseUri);
