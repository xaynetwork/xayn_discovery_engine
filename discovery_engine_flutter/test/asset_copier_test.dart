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

import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:xayn_discovery_engine_flutter/discovery_engine.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('FlutterBundleAssetCopier', () {
    late FlutterBundleAssetCopier copier;
    var loadAssetCounter = 0;
    const bundleAssetsPath = 'test/utils/assets';
    final outputPath = '${Directory.current.path}/test/tmp';
    final manifest = Manifest.fromJson({
      'assets': [
        {
          'id': 'smbertVocab',
          'url_suffix': 'dummy-asset',
          'checksum':
              'd9b2aefb1febe2dd6e403f634e18917a8c0dd1a440c976e9fe126b465ae9fc8d',
          'fragments': <Map<String, String>>[],
        },
      ]
    });

    setUp(() {
      copier = FlutterBundleAssetCopier(
        appDir: outputPath,
        bundleAssetsPath: bundleAssetsPath,
        loadAsset: (String path) async {
          loadAssetCounter++;

          final fileRef = File(path);

          if (!fileRef.existsSync()) {
            throw ArgumentError('No asset under path: $path');
          }

          final bytes = await fileRef.readAsBytes();
          return bytes.buffer.asByteData();
        },
      );
    });

    tearDown(() {
      loadAssetCounter = 0;
      final dir = Directory(outputPath);
      if (dir.existsSync()) {
        dir.deleteSync(recursive: true);
      }
    });

    test(
        'when calling "copyAssets" method and the asset file is NOT in the '
        'destination directory, then it should copy it from assets, but if '
        'the file is present, then it should NOT copy it', () async {
      final fileRef = File('$outputPath/$kAssetsPath/dummy-asset');
      await copier.copyAssets(manifest);

      expect(loadAssetCounter, equals(1));
      expect(fileRef.existsSync(), isTrue);

      // now the file exists so it shouldn't copy it again
      await copier.copyAssets(manifest);

      // counter should be still at "one"
      expect(loadAssetCounter, equals(1));
    });
  });
}
