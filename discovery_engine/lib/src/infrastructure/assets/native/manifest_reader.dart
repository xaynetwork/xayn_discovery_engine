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

import 'dart:io' show File;
import 'dart:isolate' show Isolate;

import 'package:xayn_discovery_engine/src/domain/assets/manifest_reader.dart'
    show ManifestReader;

class NativeManifestReader extends ManifestReader {
  @override
  Future<String> loadManifestAsString() async {
    final uri =
        Uri.parse('package:xayn_discovery_engine/assets/asset_manifest.json');
    final packageUri = await Isolate.resolvePackageUri(uri);
    return File(packageUri?.toFilePath() ?? '').readAsString();
  }
}

ManifestReader createManifestReader() => NativeManifestReader();
