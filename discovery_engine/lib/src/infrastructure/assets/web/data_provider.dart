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

import 'package:xayn_discovery_engine/src/domain/assets/assets.dart'
    show
        AssetType,
        AssetFetcher,
        AssetReporter,
        DataProvider,
        ManifestReader,
        SetupData;

class WebDataProvider extends DataProvider {
  @override
  final AssetFetcher assetFetcher;
  @override
  final AssetReporter assetReporter;
  @override
  final ManifestReader manifestReader;

  WebDataProvider(
    this.assetFetcher,
    this.assetReporter,
    this.manifestReader,
  );

  @override
  Future<SetupData> getSetupData() async {
    final fetched = <AssetType, Uint8List>{};
    final manifest = await manifestReader.read();

    assetReporter.fetchingStarted(manifest);

    for (final asset in manifest.assets) {
      final bytes = await assetFetcher.fetchAsset(
        asset,
        onFetched: assetReporter.assetFetched,
      );
      fetched.putIfAbsent(asset.id, () => bytes);
    }

    await assetReporter.fetchingFinished();

    return WebSetupData(
      smbertVocab: fetched[AssetType.smbertVocab]!,
      smbertModel: fetched[AssetType.smbertModel]!,
      kpeVocab: fetched[AssetType.kpeVocab]!,
      kpeModel: fetched[AssetType.kpeModel]!,
      kpeCnn: fetched[AssetType.kpeCnn]!,
      kpeClassifier: fetched[AssetType.kpeClassifier]!,
    );
  }
}

class WebSetupData extends SetupData {
  @override
  final Uint8List smbertVocab;
  @override
  final Uint8List smbertModel;
  @override
  final Uint8List kpeVocab;
  @override
  final Uint8List kpeModel;
  @override
  final Uint8List kpeCnn;
  @override
  final Uint8List kpeClassifier;

  WebSetupData({
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
  final AssetReporter assetReporter,
  final ManifestReader manifestReader,
  final String storageDirectoryPath,
) =>
    WebDataProvider(assetFetcher, assetReporter, manifestReader);
