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
