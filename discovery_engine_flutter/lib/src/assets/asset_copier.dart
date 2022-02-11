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

import 'package:flutter/services.dart' show rootBundle;
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show Manifest, kAssetsPath;

class FlutterAiAssetCopier {
  final String bundleAssetsPath;
  final String appDir;

  FlutterAiAssetCopier({
    required this.appDir,
    required this.bundleAssetsPath,
  })  : assert(appDir.isNotEmpty),
        assert(bundleAssetsPath.isNotEmpty);

  Future<void> copyAssets(Manifest manifest) async {
    final storageDirPath = '$appDir/$kAssetsPath';

    for (final asset in manifest.assets) {
      final urlSuffix = asset.urlSuffix;
      final fileRef = File('$storageDirPath/$urlSuffix');

      if (fileRef.existsSync()) continue;

      final bytes = await rootBundle.load('$bundleAssetsPath/$urlSuffix');
      await Directory(fileRef.parent.path).create(recursive: true);
      await fileRef.writeAsBytes(bytes.buffer.asUint8List(
        bytes.offsetInBytes,
        bytes.lengthInBytes,
      ));
    }
  }
}
