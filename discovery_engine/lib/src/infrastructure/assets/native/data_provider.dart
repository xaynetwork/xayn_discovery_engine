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
import 'package:xayn_discovery_engine/src/logger.dart' show logger;

const _baseAssetsPath = 'assets';

class NativeDataProvider extends DataProvider {
  @override
  final AssetFetcher assetFetcher;
  @override
  final ManifestReader manifestReader;
  final String storageDirectoryPath;

  NativeDataProvider(
    this.assetFetcher,
    this.manifestReader,
    this.storageDirectoryPath,
  );

  String get baseDirectoryPath =>
      DataProvider.joinPaths([storageDirectoryPath, _baseAssetsPath]);

  @override
  Future<SetupData> getSetupData() async {
    final paths = <AssetType, String>{};

    final manifest = await manifestReader.read();

    for (final asset in manifest.assets) {
      final path = await _getData(asset);
      paths.putIfAbsent(asset.id, () => path);
    }

    return NativeSetupData(
      smbertVocab: paths[AssetType.smbertVocab]!,
      smbertModel: paths[AssetType.smbertModel]!,
      // TODO: make sure they are not null when updating manifest json
      kpeVocab: paths[AssetType.kpeVocab] ?? '',
      kpeModel: paths[AssetType.kpeModel] ?? '',
      kpeCnn: paths[AssetType.kpeCnn] ?? '',
      kpeClassifier: paths[AssetType.kpeClassifier] ?? '',
    );
  }

  /// Returns the path to the data, if the data is not on disk yet
  /// it will be copied from the bundle to the disk.
  Future<String> _getData(Asset asset) async {
    logger.i('DataProvider: get asset data for asset id: ${asset.id}');

    final filePath =
        DataProvider.joinPaths([baseDirectoryPath, asset.urlSuffix]);
    final assetFile = File(filePath);
    final diskDirPath = assetFile.parent.path;
    await Directory(diskDirPath).create(recursive: true);

    // Only write the data on disk if the file does not exist or the checksum does not match.
    // The last check is useful in case the app is closed before we can finish to write,
    // and it can be also useful during development to test with different models.
    var doesExist = assetFile.existsSync();

    if (doesExist &&
        !await _verifyChecksum(assetFile, asset.checksum.checksumAsHex)) {
      await assetFile.delete();
      doesExist = false;
    }

    if (!doesExist) {
      final bytes = await assetFetcher.fetchAsset(asset);
      await assetFile.writeAsBytes(bytes, flush: true);
    }

    logger.i('DataProvider: asset saved under path:\n${assetFile.path}');

    return assetFile.path;
  }

  Future<bool> _verifyChecksum(File file, String checksum) async {
    final digest = await sha256.bind(file.openRead()).first;
    return digest.toString() == checksum;
  }
}

class NativeSetupData extends SetupData {
  @override
  final String smbertVocab;
  @override
  final String smbertModel;
  @override
  final String kpeVocab;
  @override
  final String kpeModel;
  @override
  final String kpeCnn;
  @override
  final String kpeClassifier;

  NativeSetupData({
    required this.smbertVocab,
    required this.smbertModel,
    required this.kpeVocab,
    required this.kpeModel,
    required this.kpeCnn,
    required this.kpeClassifier,
  });
}

DataProvider createDataProvider(
  final AssetFetcher assetFetcher,
  final ManifestReader manifestReader,
  final String storageDirectoryPath,
) =>
    NativeDataProvider(assetFetcher, manifestReader, storageDirectoryPath);
