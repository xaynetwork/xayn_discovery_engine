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
import 'utils/local_asset_server.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('DiscoveryEngine init', () {
    const port = 8080;
    late LocalAssetServer server;
    final outputPath = '${Directory.current.path}/test/tmp';

    setUpAll(() async {
      server = await LocalAssetServer.start(
        port: port,
        mockDataPath: '/test/utils/assets',
      );
    });

    tearDown(() {
      final dir = Directory(outputPath);
      if (dir.existsSync()) {
        dir.deleteSync(recursive: true);
      }
    });

    tearDownAll(() {
      server.close();
    });

    test(
        'when calling "FlutterManifestReader" read method it will return '
        'a Manifest successfully', () async {
      expect(
        FlutterManifestReader().read(),
        completion(isA<Manifest>()),
      );
    });

    test(
        'when calling DiscoveryEngine "init" method with a proper configuration '
        'it will initialize the engine and return it\'s instance', () async {
      final assets = [
        'smbertVocab',
        'smbertModel',
        'kpeVocab',
        'kpeModel',
        'kpeCnn',
        'kpeClassifier',
        'availableSources',
      ]
          .map(
            (id) => {
              'id': id,
              'url_suffix': 'dummy-asset',
              'checksum':
                  'd9b2aefb1febe2dd6e403f634e18917a8c0dd1a440c976e9fe126b465ae9fc8d',
              'fragments': <Map<String, String>>[],
            },
          )
          .toList();

      final manifest = Manifest.fromJson({'assets': assets});
      final config = Configuration(
        apiKey: 'use-mock-engine',
        apiBaseUrl: 'https://use-mock-engine.test',
        assetsUrl: 'http://127.0.0.1:$port',
        maxItemsPerFeedBatch: 50,
        maxItemsPerSearchBatch: 20,
        applicationDirectoryPath: outputPath,
        feedMarkets: {const FeedMarket(countryCode: 'DE', langCode: 'de')},
        manifest: manifest,
        headlinesProviderPath: '/newscatcher/v1/latest-headlines',
        newsProviderPath: '/newscatcher/v1/search-news',
      );

      expect(
        DiscoveryEngine.init(configuration: config),
        completion(isA<DiscoveryEngine>()),
      );
    });
  });
}
