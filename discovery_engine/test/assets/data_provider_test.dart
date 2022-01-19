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
import 'package:xayn_discovery_engine/src/domain/assets/asset.dart';
import 'package:xayn_discovery_engine/src/infrastructure/assets/data_provider.dart'
    show createDataProvider;
import 'package:xayn_discovery_engine/src/infrastructure/assets/http_asset_fetcher.dart'
    show HttpAssetFetcher;
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/data_provider.dart'
    show NativeSetupData;

import 'utils/local_asset_server.dart' show LocalAssetServer;
import 'utils/mock_http_asset_fetcher.dart' show MockHttpAssetFetcher;
import 'utils/mock_manifest_reader.dart'
    show MockManifestReader, goodJson, wrongChecksumJson;

void main() {
  group('DataProvider', () {
    group('getSetupData', () {
      final outputPath = '${Directory.current.path}/test/assets/utils/output';
      final vocabPath = '$outputPath/assets/smbert_v0000/vocab.txt';
      final modelPath = '$outputPath/assets/smbert_v0000/smbert.onnx';

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

      test(
          'when the assets were already downloaded and the checksums are matching '
          'it will serve those assets instead of fetching them again',
          () async {
        await _prepareOutputFiles(
          json: goodJson,
          vocabPath: vocabPath,
          modelPath: modelPath,
        );

        final mockAssetFetcher = MockHttpAssetFetcher('http://localhost:8080');
        final dataProvider = createDataProvider(
          mockAssetFetcher,
          MockManifestReader(goodJson),
          outputPath,
        );

        await dataProvider.getSetupData();

        expect(mockAssetFetcher.callCount, equals(0));
      });

      test(
          'when the assets were already downloaded but the checksums '
          'are NOT matching, it will fetch new files from the server',
          () async {
        await _prepareOutputFiles(
          json: wrongChecksumJson,
          vocabPath: vocabPath,
          modelPath: modelPath,
        );

        final mockAssetFetcher = MockHttpAssetFetcher('http://localhost:8080');
        final dataProvider = createDataProvider(
          mockAssetFetcher,
          MockManifestReader(wrongChecksumJson),
          outputPath,
        );

        await dataProvider.getSetupData();

        expect(mockAssetFetcher.callCount, equals(4));
      });
    });
  });
}

Future<void> _prepareOutputFiles({
  required Map<String, Object> json,
  required String vocabPath,
  required String modelPath,
}) async {
  final realAssetFetcher = HttpAssetFetcher('http://localhost:8080');
  final manifestReader = MockManifestReader(json);
  final manifest = await manifestReader.read();

  await _fetchFile(realAssetFetcher, manifest.assets.first, vocabPath);
  await _fetchFile(realAssetFetcher, manifest.assets.last, modelPath);
}

Future<void> _fetchFile(
  HttpAssetFetcher fetcher,
  Asset asset,
  String filePath,
) async {
  final bytes = await fetcher.fetchAsset(asset);
  final file = File(filePath)..createSync(recursive: true);
  await file.writeAsBytes(bytes, flush: true);
}
