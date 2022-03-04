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
        FeedFailureReason,
        NextFeedBatchRequestFailed,
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
      expect(
        engine.engineEvents,
        emitsInOrder(<Matcher>[
          isA<NextFeedBatchRequestSucceeded>(),
          isA<RestoreFeedSucceeded>(),
        ]),
      );

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
        'if a news api request error occurs, then the requestNextFeedBatch'
        ' depletes the internal stacks and subsequent calls should fail with'
        ' FeedFailureReason.noNewsForMarket', () async {
      // the server error only occurs for fetching breaking news, the personalized news succeeds
      // early with empty documents and no error before a server request is made because no key
      // phrases are selected due to no previous feedback, overall only one of the two stacks fails
      // which results in successful batch requests until all stacks are depleted
      server.replyWithError = true;

      // the next batch can still return the documents fetched during engine init
      final nextBatchResponse = await engine.requestNextFeedBatch();
      expect(nextBatchResponse, isA<NextFeedBatchRequestSucceeded>());
      expect(
        (nextBatchResponse as NextFeedBatchRequestSucceeded).items,
        isNotEmpty,
      );

      // all subsequent batches fail because of the server error
      final subsequentBatchResponse = await engine.requestNextFeedBatch();
      expect(subsequentBatchResponse, isA<NextFeedBatchRequestFailed>());
      expect(
        (subsequentBatchResponse as NextFeedBatchRequestFailed).reason,
        equals(FeedFailureReason.noNewsForMarket),
      );
    });
  });
}
