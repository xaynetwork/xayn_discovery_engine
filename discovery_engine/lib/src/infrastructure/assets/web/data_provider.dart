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

import 'package:xayn_discovery_engine/src/domain/assets/asset.dart'
    show AssetType;
import 'package:xayn_discovery_engine/src/domain/assets/asset_fetcher.dart'
    show AssetFetcher;
import 'package:xayn_discovery_engine/src/domain/assets/data_provider.dart'
    show DataProvider, SetupData;
import 'package:xayn_discovery_engine/src/domain/assets/manifest_reader.dart'
    show ManifestReader;

class WebDataProvider extends DataProvider {
  @override
  final AssetFetcher assetFetcher;
  @override
  final ManifestReader manifestReader;

  WebDataProvider(
    this.assetFetcher,
    this.manifestReader,
  );

  @override
  Future<SetupData> getSetupData() async {
    final fetched = <AssetType, Uint8List>{};
    final manifest = await manifestReader.read();

    for (final asset in manifest.assets) {
      final bytes = await assetFetcher.fetchAsset(asset);
      fetched.putIfAbsent(asset.id, () => bytes);
    }

    return WebSetupData(
      smbertVocab: fetched[AssetType.smbertVocab]!,
      smbertModel: fetched[AssetType.smbertModel]!,
      // TODO: make sure they are not null when updating manifest json
      kpeVocab: fetched[AssetType.kpeVocab] ?? Uint8List(0),
      kpeModel: fetched[AssetType.kpeModel] ?? Uint8List(0),
      kpeCnn: fetched[AssetType.kpeCnn] ?? Uint8List(0),
      kpeClassifier: fetched[AssetType.kpeClassifier] ?? Uint8List(0),
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
  final ManifestReader manifestReader,
  final String storageDirectoryPath,
) =>
    WebDataProvider(assetFetcher, manifestReader);
