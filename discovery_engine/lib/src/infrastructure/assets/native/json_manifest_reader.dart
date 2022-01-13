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
import 'dart:io' show File;

import 'package:xayn_discovery_engine/src/domain/assets/asset.dart'
    show Manifest;
import 'package:xayn_discovery_engine/src/domain/assets/reader.dart'
    show ManifestReader;

class JsonManifestReader implements ManifestReader {
  @override
  Future<Manifest> read(String path) async {
    final json = jsonDecode(await File(path).readAsString()) as Map;
    return Manifest.fromJson(json.cast<String, Object>());
  }
}
