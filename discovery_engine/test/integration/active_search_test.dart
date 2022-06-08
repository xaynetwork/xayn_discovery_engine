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
        ActiveSearchRequestFailed,
        ActiveSearchRequestSucceeded,
        ClientEventSucceeded,
        DiscoveryEngine,
        RestoreActiveSearchFailed,
        SearchFailureReason;
import 'package:xayn_discovery_engine/src/api/api.dart';

import '../logging.dart' show setupLogging;
import 'utils/helpers.dart'
    show TestEngineData, initEngine, setupTestEngineData;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

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
      expect(closeResponse, isA<ClientEventSucceeded>());

      // Restoring now should fail, as there the active search has been closed
      final restoreClosedResponse = await engine.restoreActiveSearch();
      expect(restoreClosedResponse, isA<RestoreActiveSearchFailed>());
      expect(
        (restoreClosedResponse as RestoreActiveSearchFailed).reason,
        equals(SearchFailureReason.noActiveSearch),
      );
    });

    /// TODO: This is the behaviour we want, but not what we have at the moment
    test(
      'closing when no active search is running returns an error',
      () async {
        final response = await engine.closeActiveSearch();
        expect(response, isA<EngineExceptionRaised>());
      },
      skip: true,
    );

    /// TODO: This is not currently enabled, but is intended behaviour for
    ///       the future.
    test(
      'starting a second, concurrent search throws an error',
      () async {
        final response1 = await engine.requestQuerySearch('first search');
        final response2 = await engine.requestQuerySearch('second search');

        expect(response1, isA<ActiveSearchRequestSucceeded>());
        expect(
          (response1 as ActiveSearchRequestSucceeded).items,
          isNotEmpty,
        );

        expect(response2, isA<ActiveSearchRequestFailed>());
      },
      skip: true,
    );
  });
}
