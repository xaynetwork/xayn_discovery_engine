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
    show
        AssetFetcherException,
        AssetReporter,
        DataProvider,
        Manifest,
        tmpFileExt;
import 'package:xayn_discovery_engine/src/infrastructure/assets/assets.dart'
    show createDataProvider;
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/data_provider.dart'
    show NativeSetupData;

import '../logging.dart' show setupLogging;
import 'utils/local_asset_server.dart' show LocalAssetServer, bytesMap;
import 'utils/mock_http_asset_fetcher.dart' show HttpAssetFetcherWithCounter;
import 'utils/mock_manifest_reader.dart' show goodJson, wrongChecksumJson;

void main() {
  setupLogging();

  group('DataProvider', () {
    group('getSetupData', () {
      const port = 8080;
      final outputPath = '${Directory.current.path}/test/assets/utils/output';
      final manifest = Manifest.fromJson(goodJson);
      final wrongManifest = Manifest.fromJson(wrongChecksumJson);
      final finalSetupData = NativeSetupData(
        smbertVocab: '$outputPath/smbertVocab',
        smbertModel: '$outputPath/smbertModel',
        kpeVocab: '$outputPath/kpeVocab',
        kpeModel: '$outputPath/kpeModel',
        kpeClassifier: '$outputPath/kpeClassifier',
        kpeCnn: '$outputPath/kpeCnn',
      );
      final tmpSetupData = NativeSetupData(
        smbertVocab: '$outputPath/smbertVocab.$tmpFileExt',
        smbertModel: '$outputPath/smbertModel.$tmpFileExt',
        kpeVocab: '$outputPath/kpeVocab.$tmpFileExt',
        kpeModel: '$outputPath/kpeModel.$tmpFileExt',
        kpeClassifier: '$outputPath/kpeClassifier.$tmpFileExt',
        kpeCnn: '$outputPath/kpeCnn.$tmpFileExt',
      );

      late LocalAssetServer server;
      late HttpAssetFetcherWithCounter assetFetcher;
      late AssetReporter assetReporter;
      late DataProvider dataProvider;

      setUpAll(() async {
        server = await LocalAssetServer.start(port);
      });

      setUp(() async {
        assetFetcher = HttpAssetFetcherWithCounter('http://localhost:$port');
        assetReporter = AssetReporter();
        dataProvider =
            createDataProvider(assetFetcher, assetReporter, outputPath);
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
          'when assets are not downloaded yet, and there is no temp files to '
          'verify it will fetch assets from the server and save them under '
          'specified path on the filesystem', () async {
        final setupData = await dataProvider.getSetupData(manifest);

        expect(setupData, equals(finalSetupData));
        expect(allSetupDataFilesExist(finalSetupData), isTrue);
        expect(assetFetcher.callCount, equals(8));
      });

      test(
          'when the assets were already downloaded and verified '
          'it will serve those assets instead of fetching them again',
          () async {
        await _createAssetFiles(manifest, outputPath);

        final setupData = await dataProvider.getSetupData(manifest);

        expect(setupData, equals(finalSetupData));
        expect(allSetupDataFilesExist(finalSetupData), isTrue);
        expect(assetFetcher.callCount, equals(0));
      });

      test(
          'when the temp assets are available, and they have RIGHT checksums '
          'it will verify and rename them ', () async {
        await _createAssetFiles(manifest, outputPath, isTemp: true);

        final setupData = await dataProvider.getSetupData(manifest);

        expect(setupData, equals(finalSetupData));
        expect(allSetupDataFilesExist(finalSetupData), isTrue);
        expect(allSetupDataFilesExist(tmpSetupData), isFalse);
        expect(assetFetcher.callCount, equals(0));
      });

      test(
          'when the temp assets are available, but they have WRONG checksums '
          'it will delete them and download assets from the server', () async {
        await _createAssetFiles(wrongManifest, outputPath, isTemp: true);

        final setupData = await dataProvider.getSetupData(manifest);

        expect(setupData, equals(finalSetupData));
        expect(allSetupDataFilesExist(finalSetupData), isTrue);
        expect(allSetupDataFilesExist(tmpSetupData), isFalse);
        expect(assetFetcher.callCount, equals(8));
      });

      test(
          'when fetching assets from the server, if the checksum verification '
          'of the downloaded assets fails, it will throw an '
          '"AssetFetcherException"', () async {
        expect(
          dataProvider.getSetupData(wrongManifest),
          throwsA(isA<AssetFetcherException>()),
        );
      });

      test(
          'when server responds with "503 - Service Unavailable" status '
          'the fetcher is able to retry the request', () async {
        server.setRequestFailCount(1);

        final setupData = await dataProvider.getSetupData(manifest);

        expect(setupData, equals(finalSetupData));
        expect(allSetupDataFilesExist(finalSetupData), isTrue);
        expect(server.callCountSum, equals(8));
        expect(assetFetcher.callCount, equals(8));
      });
    });
  });
}

Future<void> _createAssetFiles(
  Manifest manifest,
  String basePath, {
  bool isTemp = false,
}) async {
  await Future.wait(
    manifest.assets.map((asset) async {
      final filePath =
          '$basePath/${asset.urlSuffix}${isTemp ? '.$tmpFileExt' : ''}';
      final file = File(filePath)..createSync(recursive: true);
      final bytes =
          asset.checksum.checksum != '123' ? bytesMap[asset.urlSuffix] : [0];
      await file.writeAsBytes(bytes!, flush: true);
    }),
  );
}

bool allSetupDataFilesExist(NativeSetupData setupData) {
  final list = [
    File(setupData.smbertVocab).existsSync(),
    File(setupData.smbertModel).existsSync(),
    File(setupData.kpeVocab).existsSync(),
    File(setupData.kpeModel).existsSync(),
    File(setupData.kpeCnn).existsSync(),
    File(setupData.kpeClassifier).existsSync(),
  ];
  return list.any((it) => it == false) == false;
}
