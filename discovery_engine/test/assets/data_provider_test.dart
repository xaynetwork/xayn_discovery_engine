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
import 'package:xayn_discovery_engine/src/domain/assets/assets.dart'
    show AssetReporter, Manifest;
import 'package:xayn_discovery_engine/src/infrastructure/assets/assets.dart'
    show createDataProvider;
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/data_provider.dart'
    show NativeSetupData;

import '../logging.dart' show setupLogging;
import 'utils/local_asset_server.dart' show LocalAssetServer;
import 'utils/mock_http_asset_fetcher.dart' show HttpAssetFetcherWithCounter;
import 'utils/mock_manifest_reader.dart'
    show MockManifestReader, goodJson, wrongChecksumJson;

void main() {
  setupLogging();

  group('DataProvider', () {
    group('getSetupData', () {
      const port = 8080;
      const assetUrl = 'http://localhost:$port';
      final outputPath = '${Directory.current.path}/test/assets/utils/output';
      final dummyAssetPath = '$outputPath/dummy-asset';
      final assetFetcher = HttpAssetFetcherWithCounter(assetUrl);
      final manifest = Manifest.fromJson(goodJson);

      late LocalAssetServer server;
      late AssetReporter assetReporter;

      setUpAll(() async {
        server = await LocalAssetServer.start(port);
      });

      setUp(() async {
        assetReporter = AssetReporter();
      });

      tearDown(() {
        assetFetcher.resetCount();
        server.resetRequestFailCount();
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
          assetFetcher,
          assetReporter,
          outputPath,
        );

        final setupData =
            (await dataProvider.getSetupData(manifest)) as NativeSetupData;

        expect(File(dummyAssetPath).existsSync(), isTrue);
        expect(setupData.smbertVocab, equals(dummyAssetPath));
        expect(setupData.smbertModel, equals(dummyAssetPath));
        expect(setupData.kpeVocab, equals(dummyAssetPath));
        expect(setupData.kpeModel, equals(dummyAssetPath));
        expect(setupData.kpeCnn, equals(dummyAssetPath));
        expect(setupData.kpeClassifier, equals(dummyAssetPath));
      });

      test(
          'when the assets were already downloaded and the checksums are matching '
          'it will serve those assets instead of fetching them again',
          () async {
        final dataProvider = createDataProvider(
          assetFetcher,
          assetReporter,
          outputPath,
        );
        await _prepareOutputFiles(assetFetcher, manifest, outputPath);
        await dataProvider.getSetupData(manifest);

        expect(assetFetcher.callCount, equals(0));
      });

      test(
          'when the assets were already downloaded but the checksums '
          'are NOT matching, it will fetch new files from the server',
          () async {
        final manifestReader = MockManifestReader(wrongChecksumJson);
        final dataProvider = createDataProvider(
          assetFetcher,
          assetReporter,
          outputPath,
        );
        final manifest = await manifestReader.read();
        await _prepareOutputFiles(assetFetcher, manifest, outputPath);

        await dataProvider.getSetupData(manifest);

        expect(assetFetcher.callCount, equals(8));
      });

      test(
          'when server responds with "503 - Service Unavailable" status '
          'the fetcher is able to retry the request', () async {
        server.setRequestFailCount(1);

        await _prepareOutputFiles(assetFetcher, manifest, outputPath);

        expect(server.callCount.values, equals([1]));
        expect(File(dummyAssetPath).existsSync(), isTrue);
      });
    });
  });
}

Future<void> _prepareOutputFiles(
  HttpAssetFetcherWithCounter assetFetcher,
  Manifest manifest,
  String basePath,
) async {
  for (final asset in manifest.assets) {
    final bytes = await assetFetcher.fetchAsset(asset);
    final filePath = '$basePath/${asset.urlSuffix}';
    final file = File(filePath)..createSync(recursive: true);
    await file.writeAsBytes(bytes, flush: true);
    assetFetcher.resetCount();
  }
}
