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

@Timeout(Duration(seconds: 80))

import 'dart:io' show Directory;

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show
        DeepSearchRequestFailed,
        DeepSearchRequestSucceeded,
        DiscoveryEngine,
        DocumentId,
        NextFeedBatchRequestSucceeded,
        SearchFailureReason,
        cfgFeatureStorage;

import '../logging.dart' show setupLogging;
import 'utils/helpers.dart'
    show TestEngineData, initEngine, setupTestEngineData;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer, ReplyWith;

void main() {
  setupLogging();

  group('DiscoveryEngine requestDeepSearch', () {
    late LocalNewsApiServer server;
    late TestEngineData data;
    late DiscoveryEngine engine;

    late DocumentId id;

    setUp(() async {
      server = await LocalNewsApiServer.start();
      data = await setupTestEngineData();
      engine = await initEngine(data, server.port);

      expect(
        engine.engineEvents,
        emitsInOrder(<Matcher>[isA<NextFeedBatchRequestSucceeded>()]),
      );
      id = ((await engine.requestNextFeedBatch())
              as NextFeedBatchRequestSucceeded)
          .items[0]
          .documentId;
    });

    tearDown(() async {
      await engine.dispose();
      await server.close();
      await Directory(data.applicationDirectoryPath).delete(recursive: true);
    });

    test('requestDeepSearch should return documents', () async {
      expect(
        engine.engineEvents,
        emitsInOrder(<Matcher>[isA<DeepSearchRequestSucceeded>()]),
      );

      final response = await engine.requestDeepSearch(id);
      expect(response, isA<DeepSearchRequestSucceeded>());
      expect(
        (response as DeepSearchRequestSucceeded).items,
        isNotEmpty,
      );
    });

    test(
        'requestDeepSearch should return failed event if the server returns empty documents',
        () async {
      expect(
        engine.engineEvents,
        emitsInOrder(<Matcher>[isA<DeepSearchRequestFailed>()]),
      );

      server.replyWith = ReplyWith.empty;
      final response = await engine.requestDeepSearch(id);
      expect(response, isA<DeepSearchRequestFailed>());
      expect(
        (response as DeepSearchRequestFailed).reason,
        equals(SearchFailureReason.noResultsAvailable),
      );
    });

    test(
        'requestDeepSearch should return failed event if the server replies with error',
        () async {
      expect(
        engine.engineEvents,
        emitsInOrder(<Matcher>[isA<DeepSearchRequestFailed>()]),
      );

      server.replyWith = ReplyWith.error;
      final response = await engine.requestDeepSearch(id);
      expect(response, isA<DeepSearchRequestFailed>());
      expect(
        (response as DeepSearchRequestFailed).reason,
        equals(SearchFailureReason.noResultsAvailable),
      );
    });
    //TODO[pmk] fails with storage but should not
    //ignore: require_trailing_commas
  }, skip: cfgFeatureStorage);
}
