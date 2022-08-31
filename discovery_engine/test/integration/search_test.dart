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
        ActiveSearchTermRequestFailed,
        ActiveSearchTermRequestSucceeded,
        DiscoveryEngine,
        NextActiveSearchBatchRequestFailed,
        NextActiveSearchBatchRequestSucceeded,
        RestoreActiveSearchSucceeded,
        SearchBy,
        SearchFailureReason;

import '../logging.dart' show setupLogging;
import 'utils/helpers.dart'
    show TestEngineData, expectEvent, initEngine, setupTestEngineData;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

void main() {
  setupLogging();

  const searchTerm = 'birds eat seeds';
  const searchTerm2 = 'birds also eat insects';

  group('News Search', () {
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

    test('a new search can be opened', () async {
      final result = expectEvent<ActiveSearchRequestSucceeded>(
        await engine.requestQuerySearch(searchTerm),
      );

      expect(result.items, isNotEmpty);
      expect(result.search.searchBy, SearchBy.query);
      expect(result.search.searchTerm, searchTerm);

      expect(
        expectEvent<ActiveSearchTermRequestSucceeded>(
          await engine.getActiveSearchTerm(),
        ).searchTerm,
        equals(searchTerm),
      );
    });

    test('a new search can not be opened twice', () async {
      expectEvent<ActiveSearchRequestSucceeded>(
        await engine.requestQuerySearch(searchTerm),
      );

      expect(
        expectEvent<ActiveSearchRequestFailed>(
          await engine.requestQuerySearch(searchTerm2),
        ).reason,
        equals(SearchFailureReason.openActiveSearch),
      );
    });

    test('we can request more results until we run out of results', () async {
      final result = expectEvent<ActiveSearchRequestSucceeded>(
        await engine.requestQuerySearch(searchTerm),
      );

      expect(result.items, isNotEmpty);
      expect(result.search.searchBy, SearchBy.query);
      expect(result.search.searchTerm, searchTerm);

      final nextResult = expectEvent<NextActiveSearchBatchRequestSucceeded>(
        await engine.requestNextActiveSearchBatch(),
      );

      expect(nextResult.items, isNot(result.items));
      expect(nextResult.items, isNotEmpty);
      expect(nextResult.search.searchBy, SearchBy.query);
      expect(nextResult.search.searchTerm, searchTerm);

      expect(
        expectEvent<NextActiveSearchBatchRequestSucceeded>(
          await engine.requestNextActiveSearchBatch(),
        ).items,
        isEmpty,
      );
    });

    test('closing a search works', () async {
      final result = expectEvent<ActiveSearchRequestSucceeded>(
        await engine.requestQuerySearch(searchTerm),
      );

      expect(result.items, isNotEmpty);
      expect(result.search.searchBy, SearchBy.query);
      expect(result.search.searchTerm, searchTerm);

      expectEvent<ActiveSearchClosedSucceeded>(
        await engine.closeActiveSearch(),
      );

      expect(
        expectEvent<ActiveSearchTermRequestFailed>(
          await engine.getActiveSearchTerm(),
        ).reason,
        SearchFailureReason.noActiveSearch,
      );

      expect(
        expectEvent<NextActiveSearchBatchRequestFailed>(
          await engine.requestNextActiveSearchBatch(),
        ).reason,
        SearchFailureReason.noActiveSearch,
      );

      expect(
        expectEvent<ActiveSearchClosedFailed>(
          await engine.closeActiveSearch(),
        ).reason,
        SearchFailureReason.noActiveSearch,
      );
    });

    test('we can restore the search', () async {
      final result = expectEvent<ActiveSearchRequestSucceeded>(
        await engine.requestQuerySearch(searchTerm),
      );

      expect(result.items, isNotEmpty);
      expect(result.search.searchBy, SearchBy.query);
      expect(result.search.searchTerm, searchTerm);

      final restored = expectEvent<RestoreActiveSearchSucceeded>(
        await engine.restoreActiveSearch(),
      );

      expect(restored.items, equals(result.items));
      expect(restored.search, equals(result.search));
    });
  });

  group('News Search with persistence', () {
    late LocalNewsApiServer server;
    late TestEngineData data;
    late DiscoveryEngine engine;

    setUp(() async {
      data = await setupTestEngineData(useInMemoryDb: false);
      server = await LocalNewsApiServer.start();
      engine = await initEngine(data, server.port);
    });

    tearDown(() async {
      await engine.dispose();
      await server.close();
      await Directory(data.applicationDirectoryPath).delete(recursive: true);
    });

    test('search persists across instances', () async {
      final result = expectEvent<ActiveSearchRequestSucceeded>(
        await engine.requestQuerySearch(searchTerm),
      );

      expect(result.items, isNotEmpty);
      expect(result.search.searchBy, SearchBy.query);
      expect(result.search.searchTerm, searchTerm);

      await engine.dispose();
      engine = await initEngine(data, server.port);

      expect(
        expectEvent<ActiveSearchTermRequestSucceeded>(
          await engine.getActiveSearchTerm(),
        ).searchTerm,
        equals(searchTerm),
      );

      final nextResult = expectEvent<NextActiveSearchBatchRequestSucceeded>(
        await engine.requestNextActiveSearchBatch(),
      );

      expect(nextResult.items, isNot(result.items));
      expect(nextResult.items, isNotEmpty);
      expect(nextResult.search.searchBy, SearchBy.query);
      expect(nextResult.search.searchTerm, searchTerm);

      await engine.dispose();
      engine = await initEngine(data, server.port);

      expect(
        expectEvent<NextActiveSearchBatchRequestSucceeded>(
          await engine.requestNextActiveSearchBatch(),
        ).items,
        isEmpty,
      );

      expectEvent<ActiveSearchClosedSucceeded>(
        await engine.closeActiveSearch(),
      );

      await engine.dispose();
      engine = await initEngine(data, server.port);

      expect(
        expectEvent<ActiveSearchTermRequestFailed>(
          await engine.getActiveSearchTerm(),
        ).reason,
        SearchFailureReason.noActiveSearch,
      );
    });
  });
}
