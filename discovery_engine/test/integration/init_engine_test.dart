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

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show Configuration, DiscoveryEngine, FeedMarket, createManifestReader;

import '../logging.dart' show setupLogging;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

final appDirPath = '${Directory.current.path}/test/integration';
final engineDataPath = '$appDirPath/data';

void main() {
  setupLogging();

  group('DiscoveryEngine init', () {
    const port = 9090;
    const newsApiUrl = 'http://localhost:$port';

    late LocalNewsApiServer server;
    late Configuration config;

    setUpAll(() async {
      try {
        await Directory(engineDataPath).delete(recursive: true);
      } catch (_) {}
      await Directory(engineDataPath).create();

      await Link(
        '$engineDataPath/engine_data/assets',
      ).create(
        '${Directory.current.path}/../discovery_engine_flutter/example/assets',
        recursive: true,
      );

      final manifest = await createManifestReader().read();

      config = Configuration(
        apiKey: '**********',
        apiBaseUrl: newsApiUrl,
        assetsUrl: 'https://ai-assets.xaynet.dev',
        maxItemsPerFeedBatch: 50,
        applicationDirectoryPath: engineDataPath,
        feedMarkets: {const FeedMarket(countryCode: 'DE', langCode: 'de')},
        manifest: manifest,
      );
    });

    tearDownAll(() async {
      await Directory(engineDataPath).delete(recursive: true);
    });

    tearDown(() async {
      await server.close();
    });

    test('Init engine with ai models', () async {
      server = await LocalNewsApiServer.start(port);
      final engine = await DiscoveryEngine.init(configuration: config);

      expect(engine, isA<DiscoveryEngine>());
    });

    test('news api request error should not raise an engine exception',
        () async {
      server = await LocalNewsApiServer.start(port);
      server.replyWithError = true;
      final engine = await DiscoveryEngine.init(configuration: config);

      expect(engine, isA<DiscoveryEngine>());
    });
  });
}
