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
    show
        ClientEventSucceeded,
        DiscoveryEngine,
        ExcludedSourcesListRequestSucceeded,
        FeedFailureReason,
        NextFeedBatchRequestFailed,
        NextFeedBatchRequestSucceeded;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show Source;

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

    test('addSourceToExcludedList adds excluded source', () async {
      engine = await initEngine(data, server.port);

      final exclude = Source('example.com');

      var response = await engine.addSourceToExcludedList(exclude);
      expect(response, isA<ClientEventSucceeded>());

      final excluded = await engine.getExcludedSourcesList();
      expect(excluded, isA<ExcludedSourcesListRequestSucceeded>());
      expect(
        (excluded as ExcludedSourcesListRequestSucceeded).excludedSources,
        equals({exclude}),
      );

      response = await engine.requestNextFeedBatch();
      expect(response, isA<NextFeedBatchRequestFailed>());
      expect(
        (response as NextFeedBatchRequestFailed).reason,
        FeedFailureReason.noNewsForMarket,
      );

      expect(
        server.lastUri?.queryParameters['not_sources'],
        equals(exclude.toString()),
      );
    });

    test('removeSourceFromExcludedList removes the added excluded source',
        () async {
      engine = await initEngine(data, server.port);

      var response =
          await engine.addSourceToExcludedList(Source('example.com'));
      expect(response, isA<ClientEventSucceeded>());

      var excluded = await engine.getExcludedSourcesList();
      expect(excluded, isA<ExcludedSourcesListRequestSucceeded>());
      expect(
        (excluded as ExcludedSourcesListRequestSucceeded).excludedSources,
        equals({Source('example.com')}),
      );

      response =
          await engine.removeSourceFromExcludedList(Source('example.com'));
      expect(response, isA<ClientEventSucceeded>());

      excluded = await engine.getExcludedSourcesList();
      expect(excluded, isA<ExcludedSourcesListRequestSucceeded>());
      expect(
        (excluded as ExcludedSourcesListRequestSucceeded).excludedSources,
        isEmpty,
      );

      response = await engine.requestNextFeedBatch();
      expect(response, isA<NextFeedBatchRequestSucceeded>());
      expect((response as NextFeedBatchRequestSucceeded).items, isNotEmpty);

      expect(server.lastUri, isNotNull);
      expect(
        server.lastUri?.queryParameters['not_sources'] ?? '',
        isEmpty,
      );
    });

    test('non-existent excluded source should have no effect', () async {
      engine = await initEngine(data, server.port);

      var response =
          await engine.addSourceToExcludedList(Source('example.org'));
      expect(response, isA<ClientEventSucceeded>());

      response = await engine.requestNextFeedBatch();
      expect(response, isA<NextFeedBatchRequestSucceeded>());
      expect((response as NextFeedBatchRequestSucceeded).items, isNotEmpty);
    });
  });
}
