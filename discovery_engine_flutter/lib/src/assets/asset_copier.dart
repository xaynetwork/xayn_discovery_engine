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

import 'dart:io' show Directory, File;
import 'dart:typed_data' show ByteData;

import 'package:flutter/services.dart' show rootBundle;
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show Manifest, kAssetsPath, discoveryEngineLogger;

/// A signature for a function loading the assets from bundle.
typedef AssetLoader = Future<ByteData> Function(String key);

class FlutterBundleAssetCopier {
  final String _bundleAssetsPath;
  final String _appDir;
  final AssetLoader _loadAsset;

  FlutterBundleAssetCopier({
    required String appDir,
    required String bundleAssetsPath,
    AssetLoader? loadAsset,
  })  : assert(appDir.isNotEmpty),
        assert(bundleAssetsPath.isNotEmpty),
        _appDir = appDir,
        _bundleAssetsPath = bundleAssetsPath,
        _loadAsset = loadAsset ?? rootBundle.load;

  Future<void> copyAssets(Manifest manifest) async {
    final storageDirPath = '$_appDir/$kAssetsPath';

    for (final asset in manifest.assets) {
      final urlSuffix = asset.urlSuffix;
      final fileRef = File('$storageDirPath/$urlSuffix');

      if (fileRef.existsSync()) continue;

      try {
        final bytes = await _loadAsset('$_bundleAssetsPath/$urlSuffix');
        await Directory(fileRef.parent.path).create(recursive: true);
        await fileRef.writeAsBytes(bytes.buffer.asUint8List(
          bytes.offsetInBytes,
          bytes.lengthInBytes,
        ));
      } catch (e, s) {
        final message =
            'Couldn\'t copy the asset "$urlSuffix" to the path: ${fileRef.path}';
        discoveryEngineLogger.e(message, e, s);
      }
    }
  }
}
