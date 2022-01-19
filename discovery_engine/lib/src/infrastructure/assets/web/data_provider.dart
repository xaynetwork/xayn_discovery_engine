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

const _baseAssetUrl = 'assets/assets';

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
    final fetched = <AssetType, Object>{};
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

    return WebSetupData(fetched);
  }
}

class WebSetupData extends SetupData {
  final Map<AssetType, Object> _assets;
  final Uint8List smbertVocab;
  final Uint8List smbertModel;

  @override
  Map<AssetType, Object> get assets => _assets;

  WebSetupData(this._assets)
      : smbertVocab = _assets[AssetType.smbertVocab]! as Uint8List,
        smbertModel = _assets[AssetType.smbertModel]! as Uint8List;
}

DataProvider createDataProvider(
  final AssetFetcher assetFetcher,
  final ManifestReader manifestReader,
  final String storageDirectoryPath,
) =>
    WebDataProvider(assetFetcher, manifestReader);
