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

import 'dart:io' show Directory;

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show ClientEventSucceeded, DiscoveryEngine, NextFeedBatchRequestSucceeded;

import '../logging.dart' show setupLogging;
import 'utils/helpers.dart'
    show TestEngineData, initEngine, setupTestEngineData;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

void main() {
  setupLogging();

  group('DiscoveryEngine excludedSources', () {
    late LocalNewsApiServer server;
    late TestEngineData data;
    late DiscoveryEngine engine;

    setUp(() async {
      server = await LocalNewsApiServer.start();
      data = await setupTestEngineData();
      engine = await initEngine(data, server.port);
    });

    tearDown(() async {
      await engine.dispose();
      await server.close();
      await Directory(data.applicationDirectoryPath).delete(recursive: true);
    });

    test('excludedSources should be updated in the engine', () async {
      engine = await initEngine(data, server.port);
      final nextFeedBatchResponse = await engine.requestNextFeedBatch();
      expect(nextFeedBatchResponse, isA<NextFeedBatchRequestSucceeded>());
      expect(server.lastUri?.queryParameters['not_sources'], isNull);
      var response = await engine.addSourceToExcludedList('dodo.test');
      expect(response, isA<ClientEventSucceeded>());
      expect(
        server.lastUri?.queryParameters['not_sources'],
        equals('dodo.test'),
      );
      response = await engine.addSourceToExcludedList('dada.test');
      expect(response, isA<ClientEventSucceeded>());
      response = await engine.removeSourceFromExcludedList('dodo.test');
      expect(response, isA<ClientEventSucceeded>());
      response = await engine.addSourceToExcludedList('so.yo.test');
      expect(response, isA<ClientEventSucceeded>());
      expect(
        server.lastUri?.queryParameters['not_sources']?.split(',').toSet(),
        equals({
          'dada.test',
          'so.yo.test',
        }),
      );
    });
  });
}
