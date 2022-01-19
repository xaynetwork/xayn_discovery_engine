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
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/infrastructure/assets/data_provider.dart'
    show createDataProvider;
import 'package:xayn_discovery_engine/src/infrastructure/assets/http_asset_fetcher.dart'
    show HttpAssetFetcher;
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/data_provider.dart'
    show NativeSetupData;

import 'utils/local_asset_server.dart' show LocalAssetServer;
import 'utils/mock_manifest_reader.dart' show MockManifestReader, goodJson;

void main() {
  group('DataProvider', () {
    group('getSetupData', () {
      final outputPath = '${Directory.current.path}/test/assets/utils/output';
      late LocalAssetServer server;

      setUpAll(() async {
        server = await LocalAssetServer.start();
      });

      tearDown(() {
        Directory(outputPath).deleteSync(recursive: true);
      });

      tearDownAll(() {
        server.close();
      });

      test(
          'when provided with proper json manifest it can download assets '
          'and asset fragments, and save them to a specified output path',
          () async {
        final vocabPath = '$outputPath/assets/smbert_v0000/vocab.txt';
        final modelPath = '$outputPath/assets/smbert_v0000/smbert.onnx';
        final dataProvider = createDataProvider(
          HttpAssetFetcher('http://localhost:8080'),
          MockManifestReader(goodJson),
          outputPath,
        );

        final setupData =
            (await dataProvider.getSetupData()) as NativeSetupData;

        expect(setupData.smbertVocab, equals(vocabPath));
        expect(File(vocabPath).existsSync(), isTrue);
        expect(setupData.smbertModel, equals(modelPath));
        expect(File(modelPath).existsSync(), isTrue);
      });
    });
  });
}
