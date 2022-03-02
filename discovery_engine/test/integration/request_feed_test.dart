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
    show
        DiscoveryEngine,
        EngineExceptionRaised,
        NextFeedBatchRequestSucceeded,
        RestoreFeedSucceeded;

import '../logging.dart' show setupLogging;
import 'utils/helpers.dart'
    show TestEngineData, initEngine, setupTestEngineData;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

void main() {
  setupLogging();

  group('DiscoveryEngine restoreFeed', () {
    late LocalNewsApiServer server;
    late TestEngineData data;
    late DiscoveryEngine engine;

    setUp(() async {
      data = await setupTestEngineData();
      server = await LocalNewsApiServer.start();
      engine = await initEngine(data, server.port);
    });

    tearDown(() async {
      await engine.dispose();
      await server.close();
      await Directory(data.applicationDirectoryPath).delete(recursive: true);
    });

    test(
        'restoreFeed should return the feed that has been requested before with'
        ' requestNextFeedBatch', () async {
      final nextBatchResponse = await engine.requestNextFeedBatch();
      final restoreFeedResponse = await engine.restoreFeed();

      expect(nextBatchResponse, isA<NextFeedBatchRequestSucceeded>());
      expect(restoreFeedResponse, isA<RestoreFeedSucceeded>());
      expect(
        (nextBatchResponse as NextFeedBatchRequestSucceeded).items,
        equals((restoreFeedResponse as RestoreFeedSucceeded).items),
      );
    });

    test(
        'if requestNextFeedBatch fails due to a news api request error, restoreFeed'
        ' should return an empty list', () async {
      server = await LocalNewsApiServer.start();
      final engine = await DiscoveryEngine.init(
        configuration: createConfig(data, server.port),
      );

      server.replyWithError = true;
      final nextBatchResponse = await engine.requestNextFeedBatch();
      expect(nextBatchResponse, isA<EngineExceptionRaised>());
      final restoreFeedResponse = await engine.restoreFeed();

      expect(restoreFeedResponse, isA<RestoreFeedSucceeded>());
      expect((restoreFeedResponse as RestoreFeedSucceeded).items, isEmpty);
    });
  });
}
