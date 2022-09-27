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

import 'dart:io' show File, Directory, FileSystemException;

import 'package:crypto/crypto.dart' show sha256;
import 'package:xayn_discovery_engine/src/domain/assets/assets.dart'
    show
        Asset,
        AssetFetcher,
        AssetFetcherException,
        AssetReporter,
        AssetType,
        DataProvider,
        Manifest,
        SetupData,
        tmpFileExt;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show AvailableSources;

class NativeDataProvider extends DataProvider {
  @override
  final AssetFetcher assetFetcher;
  @override
  final AssetReporter assetReporter;
  final String storageDirectoryPath;

  NativeDataProvider(
    this.assetFetcher,
    this.assetReporter,
    this.storageDirectoryPath,
  );

  @override
  Future<SetupData> getSetupData(Manifest manifest) async {
    final paths = <AssetType, String>{};
    assetReporter.fetchingStarted(manifest);

    for (final asset in manifest.assets) {
      final path = await _getData(asset);
      paths.putIfAbsent(asset.id, () => path);
    }

    await assetReporter.fetchingFinished();

    return NativeSetupData(
      smbertVocab: paths[AssetType.smbertVocab]!,
      smbertModel: paths[AssetType.smbertModel]!,
      availableSources: paths[AssetType.availableSources]!,
    );
  }

  /// Returns the path to the data, if the data is not on disk yet
  /// it will be copied from the bundle to the disk.
  Future<String> _getData(Asset asset) async {
    final filePath =
        DataProvider.joinPaths([storageDirectoryPath, asset.urlSuffix]);
    final assetFileRef = File(filePath);
    await Directory(assetFileRef.parent.path).create(recursive: true);

    // if the file exists it means it was verified and has a proper checksum
    if (assetFileRef.existsSync()) {
      if (asset.fragments.isEmpty) {
        assetReporter.assetFetched(asset.urlSuffix);
      }
      for (final fragment in asset.fragments) {
        assetReporter.assetFetched(fragment.urlSuffix);
      }
      return assetFileRef.path;
    }

    // if the file doesn't exist we try to look for the temp file copied from
    // bundled assets, waiting for checksum verification
    final tmpFileRef = File('$filePath.$tmpFileExt');

    try {
      if (await _verifyChecksum(tmpFileRef, asset.checksum.checksumAsHex)) {
        // if we have a tmp file and it has a proper checksum we move it
        // to proper destination path
        await tmpFileRef.rename(filePath);
        return assetFileRef.path;
      } else {
        // if the verification fails it's better to remove the tmp file to skip
        // this step and start fetching from server faster
        await tmpFileRef.delete();
      }
    } on FileSystemException {
      // file probably doesn't exist, so just continue
    }

    // if we didn't found a tmp file or it's verification failed we try to fetch
    // the asset from the server and write it to disk
    final bytes = await assetFetcher.fetchAsset(
      asset,
      onFetched: assetReporter.assetFetched,
    );
    await assetFileRef.writeAsBytes(bytes, flush: true);

    // we check if it has a proper checksum, if not we throw
    if (!await _verifyChecksum(assetFileRef, asset.checksum.checksumAsHex)) {
      await assetFileRef.delete();
      throw AssetFetcherException(
        'Asset: "${asset.urlSuffix}" failed checksum verification.',
      );
    }

    return assetFileRef.path;
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
  final String availableSources;

  NativeSetupData({
    required this.smbertVocab,
    required this.smbertModel,
    required this.availableSources,
  });

  @override
  Future<AvailableSources> getAvailableSources() async =>
      AvailableSources.fromBytes(File(availableSources).openRead());
}

DataProvider createDataProvider(
  final AssetFetcher assetFetcher,
  final AssetReporter assetReporter,
  final String storageDirectoryPath,
) =>
    NativeDataProvider(assetFetcher, assetReporter, storageDirectoryPath);
