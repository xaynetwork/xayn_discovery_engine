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

import 'dart:convert' show jsonDecode;
import 'package:xayn_discovery_engine/src/domain/assets/asset.dart'
    show Manifest;

abstract class ManifestReader {
  /// Loads and returns the assets [Manifest].
  Future<Manifest> read() async {
    final jsonString = await loadManifestAsString();
    final json = jsonDecode(jsonString) as Map;
    return Manifest.fromJson(json.cast<String, Object>());
  }

  /// Loads the [Manifest] json file as [String] from bundled assets.
  Future<String> loadManifestAsString();
}

/// Thrown when a there is an issue reading the assets manifest file.
class ManifestReaderException implements Exception {
  /// Message (or string representation of the exception).
  final String message;

  ManifestReaderException(this.message);

  @override
  String toString() => 'ManifestReaderException: $message';
}
