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
        ActiveSearchClosedFailed,
        ActiveSearchClosedSucceeded,
        ActiveSearchRequestFailed,
        ActiveSearchRequestSucceeded,
        DiscoveryEngine,
        RestoreActiveSearchFailed,
        SearchFailureReason,
        cfgFeatureStorage;
import 'package:xayn_discovery_engine/src/api/api.dart';

import '../logging.dart' show setupLogging;
import 'utils/helpers.dart'
    show TestEngineData, initEngine, setupTestEngineData;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer, ReplyWith;

void main() {
  setupLogging();

  group('DiscoveryEngine active search test', () {
    late LocalNewsApiServer server;
    late TestEngineData data;
    late DiscoveryEngine engine;

    setUp(() async {
      data = await setupTestEngineData();
      server = await LocalNewsApiServer.start();
      engine = await initEngine(data, server.port);
      expect(engine, isA<DiscoveryEngine>());
    });

    tearDown(() async {
      await engine.dispose();
      await server.close();
      await Directory(data.applicationDirectoryPath).delete(recursive: true);
    });

    test('basic search works', () async {
      final response = await engine.requestQuerySearch('some search query');
      expect(response, isA<ActiveSearchRequestSucceeded>());

      expect(
        (response as ActiveSearchRequestSucceeded).items,
        isNotEmpty,
      );

      expect(
        response.search.searchTerm,
        equals('some search query'),
      );
    });

    test('restoring when no active search is running returns an error',
        () async {
      final response = await engine.restoreActiveSearch();
      expect(response, isA<RestoreActiveSearchFailed>());
      expect(
        (response as RestoreActiveSearchFailed).reason,
        equals(SearchFailureReason.noActiveSearch),
      );
    });

    test('restoring a search works', () async {
      // In order to restore a search, we first need to initiate one
      final searchResponse =
          await engine.requestQuerySearch('some search query');
      expect(searchResponse, isA<ActiveSearchRequestSucceeded>());
      final items = (searchResponse as ActiveSearchRequestSucceeded).items;
      expect(items, isNotEmpty);

      // Let's restore the search and see if it's the same we just sent
      final restoreResponse = await engine.restoreActiveSearch();
      expect(restoreResponse, isA<RestoreActiveSearchSucceeded>());
      final succeededResponse = restoreResponse as RestoreActiveSearchSucceeded;

      expect(succeededResponse.search.searchTerm, equals('some search query'));
      expect(succeededResponse.items, equals(items));
    });

    test('closing a search works', () async {
      // In order to test closing a search, we need to initiate one first
      final searchResponse =
          await engine.requestQuerySearch('some search query');
      expect(searchResponse, isA<ActiveSearchRequestSucceeded>());

      // Since the search is still open, we should be able to restore it
      final restoreResponse = await engine.restoreActiveSearch();
      expect(restoreResponse, isA<RestoreActiveSearchSucceeded>());

      // Closing the search should work, since a search is active
      final closeResponse = await engine.closeActiveSearch();
      expect(closeResponse, isA<ActiveSearchClosedSucceeded>());

      // Restoring now should fail, as there the active search has been closed
      final restoreClosedResponse = await engine.restoreActiveSearch();
      expect(restoreClosedResponse, isA<RestoreActiveSearchFailed>());
      expect(
        (restoreClosedResponse as RestoreActiveSearchFailed).reason,
        equals(SearchFailureReason.noActiveSearch),
      );
    });

    test("request next batch doesn't work when there's no active search",
        () async {
      final nextBatchResponse = await engine.requestNextActiveSearchBatch();
      expect(nextBatchResponse, isA<NextActiveSearchBatchRequestFailed>());
      expect(
        (nextBatchResponse as NextActiveSearchBatchRequestFailed).reason,
        equals(SearchFailureReason.noActiveSearch),
      );
    });

    test('request next batch fetches new feed items', () async {
      // In order to request more items, we first need to initiate one
      final searchResponse =
          await engine.requestQuerySearch('some search query');
      expect(searchResponse, isA<ActiveSearchRequestSucceeded>());
      final items = (searchResponse as ActiveSearchRequestSucceeded).items;
      expect(items, isNotEmpty);

      server.replyWith = ReplyWith.data2;
      final nextBatchResponse = await engine.requestNextActiveSearchBatch();
      expect(nextBatchResponse, isA<NextActiveSearchBatchRequestSucceeded>());
      final succeededResponse =
          nextBatchResponse as NextActiveSearchBatchRequestSucceeded;

      expect(succeededResponse.items, isNotEmpty);
      expect(
        succeededResponse.items[0].resource.title,
        isNot(equals(items[0].resource.title)),
      );
    });

    test(
        'closing when no active search is running returns an "ActiveSearchClosedFailed" error',
        () async {
      final response = await engine.closeActiveSearch();
      expect(response, isA<ActiveSearchClosedFailed>());
      expect(
        (response as ActiveSearchClosedFailed).reason,
        equals(SearchFailureReason.noActiveSearch),
      );
    });

    test('starting a second, concurrent search throws an error', () async {
      final response1 = await engine.requestQuerySearch('first search');
      final response2 = await engine.requestQuerySearch('second search');

      expect(response1, isA<ActiveSearchRequestSucceeded>());
      expect(
        (response1 as ActiveSearchRequestSucceeded).items,
        isNotEmpty,
      );

      expect(response2, isA<ActiveSearchRequestFailed>());
      expect(
        (response2 as ActiveSearchRequestFailed).reason,
        equals(SearchFailureReason.openActiveSearch),
      );
    });
    //TODO[pmk] fails with storage but should not
    //ignore: require_trailing_commas
  }, skip: cfgFeatureStorage);
}
