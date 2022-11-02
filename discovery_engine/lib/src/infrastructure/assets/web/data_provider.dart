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
        Manifest,
        SetupData;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show AvailableSources;

class WebDataProvider extends DataProvider {
  @override
  final AssetFetcher assetFetcher;
  @override
  final AssetReporter assetReporter;

  WebDataProvider(
    this.assetFetcher,
    this.assetReporter,
  );

  @override
  Future<SetupData> getSetupData(Manifest manifest) async {
    final fetched = <AssetType, Uint8List>{};

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
      smbertConfig: fetched[AssetType.smbertConfig]!,
      smbertVocab: fetched[AssetType.smbertVocab]!,
      smbertModel: fetched[AssetType.smbertModel]!,
      availableSources: fetched[AssetType.availableSources]!,
    );
  }
}

class WebSetupData extends SetupData {
  @override
  final Uint8List smbertConfig;
  @override
  final Uint8List smbertVocab;
  @override
  final Uint8List smbertModel;
  @override
  final Uint8List availableSources;

  WebSetupData({
    required this.smbertConfig,
    required this.smbertVocab,
    required this.smbertModel,
    required this.availableSources,
  });

  @override
  Future<AvailableSources> getAvailableSources() async =>
      AvailableSources.fromBytes(Stream.value(availableSources.toList()));
}

DataProvider createDataProvider(
  final AssetFetcher assetFetcher,
  final AssetReporter assetReporter,
  final String storageDirectoryPath,
) =>
    WebDataProvider(assetFetcher, assetReporter);
