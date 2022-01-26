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

import 'package:xayn_discovery_engine/src/domain/assets/asset_fetcher.dart'
    show AssetFetcher;
import 'package:xayn_discovery_engine/src/domain/assets/manifest_reader.dart'
    show ManifestReader;

/// Data that is required to initialize [`XaynAi`].
abstract class SetupData {
  Object get smbertVocab;
  Object get smbertModel;
  Object get kpeVocab;
  Object get kpeModel;
  Object get kpeCnn;
  Object get kpeClassifier;
}

/// Reads the assets manifest and provides the [SetupData] to further use.
abstract class DataProvider {
  AssetFetcher get assetFetcher;
  ManifestReader get manifestReader;

  Future<SetupData> getSetupData() {
    throw UnsupportedError('Unsupported platform.');
  }

  static String joinPaths(List<String> paths) {
    return paths.where((e) => e.isNotEmpty).join('/');
  }
}

/// Thrown when a there is an issue with downloading AI assets.
class DataProviderException implements Exception {
  /// Message (or string representation of the exception).
  final String message;

  DataProviderException(this.message);

  @override
  String toString() => 'DataProviderException: $message';
}
