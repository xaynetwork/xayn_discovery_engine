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
import 'package:xayn_discovery_engine/src/domain/assets/manifest_reader.dart'
    show ManifestReader;
import 'package:xayn_discovery_engine/src/infrastructure/assets/data_provider.dart'
    show createDataProvider;
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/data_provider.dart'
    show NativeSetupData;

import 'utils/local_asset_server.dart' show LocalAssetServer;
import 'utils/mock_http_asset_fetcher.dart' show HttpAssetFetcherWithCounter;
import 'utils/mock_manifest_reader.dart'
    show MockManifestReader, goodJson, wrongChecksumJson;

void main() {
  group('DataProvider', () {
    group('getSetupData', () {
      final outputPath = '${Directory.current.path}/test/assets/utils/output';
      final baseAssetPath = '$outputPath/assets';
      final vocabPath = '$baseAssetPath/smbert_v0000/vocab.txt';
      final modelPath = '$baseAssetPath/smbert_v0000/smbert.onnx';
      final assetFetcher = HttpAssetFetcherWithCounter('http://localhost:8080');
      final manifestReader = MockManifestReader(goodJson);

      late LocalAssetServer server;

      setUpAll(() async {
        server = await LocalAssetServer.start();
      });

      tearDown(() {
        assetFetcher.resetCount();
        Directory(outputPath).deleteSync(recursive: true);
      });

      tearDownAll(() {
        server.close();
      });

      test(
          'when provided with proper json manifest it can download assets '
          'and asset fragments, and save them to a specified output path',
          () async {
        final dataProvider =
            createDataProvider(assetFetcher, manifestReader, outputPath);

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
        final dataProvider =
            createDataProvider(assetFetcher, manifestReader, outputPath);
        await _prepareOutputFiles(assetFetcher, manifestReader, baseAssetPath);

        await dataProvider.getSetupData();

        expect(assetFetcher.callCount, equals(0));
      });

      test(
          'when the assets were already downloaded but the checksums '
          'are NOT matching, it will fetch new files from the server',
          () async {
        final manifestReader = MockManifestReader(wrongChecksumJson);
        final dataProvider =
            createDataProvider(assetFetcher, manifestReader, outputPath);
        await _prepareOutputFiles(assetFetcher, manifestReader, baseAssetPath);

        await dataProvider.getSetupData();

        expect(assetFetcher.callCount, equals(4));
      });

      test(
          'when server responds with "503 - Service Unavailable" status '
          'the fetcher is able to retry the request', () async {
        await server.close();
        server = await LocalAssetServer.start(retryCount: 1);

        await _prepareOutputFiles(assetFetcher, manifestReader, baseAssetPath);

        expect(server.callCount.values, equals([1, 1, 1, 1]));
        expect(File(vocabPath).existsSync(), isTrue);
        expect(File(modelPath).existsSync(), isTrue);
      });
    });
  });
}

Future<void> _prepareOutputFiles(
  HttpAssetFetcherWithCounter assetFetcher,
  ManifestReader manifestReader,
  String basePath,
) async {
  final manifest = await manifestReader.read();

  for (final asset in manifest.assets) {
    final bytes = await assetFetcher.fetchAsset(asset);
    final filePath = '$basePath/${asset.urlSuffix}';
    final file = File(filePath)..createSync(recursive: true);
    await file.writeAsBytes(bytes, flush: true);
    assetFetcher.resetCount();
  }
}
