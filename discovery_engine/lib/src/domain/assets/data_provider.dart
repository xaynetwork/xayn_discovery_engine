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

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:xayn_discovery_engine/src/domain/assets/assets.dart'
    show AssetFetcher, AssetReporter, Manifest;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show AvailableSources;

const _kEnginePath = 'engine_data';
const kAssetsPath = '$_kEnginePath/assets';
const kDatabasePath = '$_kEnginePath/database';
const tmpFileExt = 'tmp';

/// Data that is required to initialize [`XaynAi`].
abstract class SetupData with EquatableMixin {
  Object get smbertVocab;
  Object get smbertModel;
  Object get availableSources;

  @override
  List<Object?> get props => [smbertVocab, smbertModel, availableSources];

  Future<AvailableSources> getAvailableSources();
}

/// Reads the assets manifest and provides the [SetupData] to further use.
abstract class DataProvider {
  AssetFetcher get assetFetcher;
  AssetReporter get assetReporter;

  Future<SetupData> getSetupData(Manifest manifest);

  static String joinPaths(List<String> paths) {
    return paths.where((e) => e.isNotEmpty).join('/');
  }
}
