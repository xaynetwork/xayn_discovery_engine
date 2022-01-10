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
