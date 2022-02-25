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
        FeedRequestSucceeded,
        NextFeedBatchRequestFailed,
        NextFeedBatchRequestSucceeded;

import '../logging.dart' show setupLogging;
import 'utils/create_config.dart'
    show TestEngineData, createConfig, setupTestEngineData;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

void main() {
  setupLogging();

  group('DiscoveryEngine requestFeed', () {
    late LocalNewsApiServer server;
    late TestEngineData data;

    setUp(() async {
      data = await setupTestEngineData();
    });

    tearDown(() async {
      await server.close();
      await Directory(data.applicationDirectoryPath).delete(recursive: true);
    });

    test(
        'requestFeed should return the feed that has been requested before with'
        'requestNextFeedBatch', () async {
      server = await LocalNewsApiServer.start();
      final engine = await DiscoveryEngine.init(
        configuration: createConfig(data, server.port),
      );

      final nextBatchResponse = await engine.requestNextFeedBatch();
      final restoreFeedResponse = await engine.requestFeed();

      expect(nextBatchResponse, isA<NextFeedBatchRequestSucceeded>());
      expect(restoreFeedResponse, isA<FeedRequestSucceeded>());
      expect(
        (nextBatchResponse as NextFeedBatchRequestSucceeded).items,
        equals((restoreFeedResponse as FeedRequestSucceeded).items),
      );
    });

    test(
        'if requestNextFeedBatch fails due to a news api request error, requestFeed'
        'should return an empty list', () async {
      server = await LocalNewsApiServer.start();
      final engine = await DiscoveryEngine.init(
        configuration: createConfig(data, server.port),
      );

      server.replyWithError = true;
      final nextBatchResponse = await engine.requestNextFeedBatch();
      expect(nextBatchResponse, isA<NextFeedBatchRequestFailed>());
      final restoreFeedResponse = await engine.requestFeed();

      expect(restoreFeedResponse, isA<FeedRequestSucceeded>());
      expect((restoreFeedResponse as FeedRequestSucceeded).items, isEmpty);
    });
  });
}
