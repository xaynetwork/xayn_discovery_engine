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

export 'package:xayn_discovery_engine/src/domain/assets/asset.dart'
    show Manifest, Asset, AssetType, Checksum, Fragment;
export 'package:xayn_discovery_engine/src/domain/assets/asset_fetcher.dart'
    show AssetFetcher, AssetFetcherException;
export 'package:xayn_discovery_engine/src/domain/assets/asset_reporter.dart'
    show AssetReporter;
export 'package:xayn_discovery_engine/src/domain/assets/data_provider.dart'
    show DataProvider, SetupData, kAssetsPath, kDatabasePath, tmpFileExt;
export 'package:xayn_discovery_engine/src/domain/assets/manifest_reader.dart'
    show ManifestReader;
