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
        NextFeedBatchRequestSucceeded;

import '../logging.dart' show setupLogging;
import 'utils/create_config.dart'
    show TestEngineData, createConfig, setupTestEngineData;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

void main() {
  setupLogging();

  group('DiscoveryEngine requestNextFeedBatch', () {
    late LocalNewsApiServer server;
    late TestEngineData data;

    setUp(() async {
      data = await setupTestEngineData();
    });

    tearDown(() async {
      await server.close();
      await Directory(data.applicationDirectoryPath).delete(recursive: true);
    });

    test('requestNextFeedBatch should return the next feed batch', () async {
      server = await LocalNewsApiServer.start();
      final engine = await DiscoveryEngine.init(
        configuration: createConfig(data, server.port),
      );

      final nextBatchResponse = await engine.requestNextFeedBatch();
      expect(nextBatchResponse, isA<NextFeedBatchRequestSucceeded>());
      expect(
        (nextBatchResponse as NextFeedBatchRequestSucceeded).items,
        isNotEmpty,
      );
    });

    test(
        'if news api request fails, requestNextFeedBatch should return the'
        'NextFeedBatchRequestFailed event with the reason stacksOpsError',
        () async {
      server = await LocalNewsApiServer.start();
      final engine = await DiscoveryEngine.init(
        configuration: createConfig(data, server.port),
      );

      server.replyWithError = true;

      final nextBatchResponse = await engine.requestNextFeedBatch();
      expect(nextBatchResponse, isA<NextFeedBatchRequestFailed>());
      expect(
        (nextBatchResponse as NextFeedBatchRequestFailed).reason,
        equals(FeedFailureReason.stacksOpsError),
      );
      expect(
        nextBatchResponse.errors,
        contains('kind: Status(404)'),
      );
    });
  });
}
