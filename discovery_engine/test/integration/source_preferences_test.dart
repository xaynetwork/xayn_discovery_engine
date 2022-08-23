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
        DiscoveryEngine,
        ExcludedSourcesListRequestSucceeded,
        FeedFailureReason,
        NextFeedBatchRequestFailed,
        NextFeedBatchRequestSucceeded;
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart';
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show Source;

import '../logging.dart' show setupLogging;
import 'utils/helpers.dart'
    show TestEngineData, initEngine, setupTestEngineData, expectEvent;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

void main() {
  setupLogging();

  final excluded = Source('foo1.example');
  final excluded2 = Source('foo2.example');
  final trusted = Source('bar1.example');
  final trusted2 = Source('bar2.example');
  final trusted3 = Source('bar3.example');
  final duplicate = Source('duplicate.example');

  group('DiscoveryEngine source preferences', () {
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
      expectEvent<AddExcludedSourceRequestSucceeded>(
        await engine.addSourceToExcludedList(excluded),
      );

      final listResponse = expectEvent<ExcludedSourcesListRequestSucceeded>(
        await engine.getExcludedSourcesList(),
      );
      expect(listResponse.excludedSources, equals({excluded}));

      final nextBatchResponse = expectEvent<NextFeedBatchRequestFailed>(
        await engine.requestNextFeedBatch(),
      );
      expect(nextBatchResponse.reason, FeedFailureReason.noNewsForMarket);

      expect(server.lastCapturedRequest, isNotNull);
      server.lastCapturedRequest!
          .expectJsonQueryParams({'not_sources': excluded.toString()});
    });

    test('removeSourceFromExcludedList removes the added excluded source',
        () async {
      expectEvent<AddExcludedSourceRequestSucceeded>(
        await engine.addSourceToExcludedList(excluded),
      );

      var listResponse = expectEvent<ExcludedSourcesListRequestSucceeded>(
        await engine.getExcludedSourcesList(),
      );
      expect(listResponse.excludedSources, equals({excluded}));

      expectEvent<RemoveExcludedSourceRequestSucceeded>(
        await engine.removeSourceFromExcludedList(excluded),
      );

      listResponse = expectEvent<ExcludedSourcesListRequestSucceeded>(
        await engine.getExcludedSourcesList(),
      );
      expect(listResponse.excludedSources, isEmpty);

      final nextBatchResponse = expectEvent<NextFeedBatchRequestSucceeded>(
        await engine.requestNextFeedBatch(),
      );
      expect(nextBatchResponse.items, isNotEmpty);

      expect(server.lastCapturedRequest, isNotNull);
      final json = server.lastCapturedRequest!.expectJsonBody();
      expect(json, isA<Map<String, Object?>>());
      final jsonMap = json as Map<String, Object?>;
      expect(
        jsonMap['not_sources'] ?? '',
        isEmpty,
      );
    });

    test('non-existent excluded source should have no effect', () async {
      expectEvent<AddExcludedSourceRequestSucceeded>(
        await engine.addSourceToExcludedList(trusted),
      );

      final nextBatchResponse = expectEvent<NextFeedBatchRequestSucceeded>(
        await engine.requestNextFeedBatch(),
      );
      expect(nextBatchResponse.items, isNotEmpty);
    });

    test('addSourceToTrustedList adds trusted source', () async {
      final addResponse = expectEvent<AddTrustedSourceRequestSucceeded>(
        await engine.addSourceToTrustedList(trusted),
      );
      expect(addResponse.source, equals(trusted));

      final listResponse = expectEvent<TrustedSourcesListRequestSucceeded>(
        await engine.getTrustedSourcesList(),
      );
      expect(listResponse.sources, equals({trusted}));
    });

    test('removeSourceFromTrustedList removes the added trusted source',
        () async {
      final someSource = Source('example.com');

      expectEvent<AddTrustedSourceRequestSucceeded>(
        await engine.addSourceToTrustedList(trusted),
      );
      expectEvent<AddTrustedSourceRequestSucceeded>(
        await engine.addSourceToTrustedList(someSource),
      );

      var listResponse = expectEvent<TrustedSourcesListRequestSucceeded>(
        await engine.getTrustedSourcesList(),
      );
      expect(
        listResponse.sources,
        equals({trusted, someSource}),
      );
      final removeResponse = expectEvent<RemoveTrustedSourceRequestSucceeded>(
        await engine.removeSourceFromTrustedList(trusted),
      );
      expect(removeResponse.source, equals(trusted));

      listResponse = expectEvent<TrustedSourcesListRequestSucceeded>(
        await engine.getTrustedSourcesList(),
      );
      expect(
        listResponse.sources,
        equals({someSource}),
      );
    });

    test('setSources', () async {
      final response1 = await engine.setSources(
        trustedSources: {trusted},
        excludedSources: {excluded},
      );

      expect(response1, isA<SetSourcesRequestSucceeded>());
      expect(
        expectEvent<TrustedSourcesListRequestSucceeded>(
          await engine.getTrustedSourcesList(),
        ).sources,
        equals({trusted}),
      );
      expect(
        expectEvent<ExcludedSourcesListRequestSucceeded>(
          await engine.getExcludedSourcesList(),
        ).excludedSources,
        equals({excluded}),
      );

      final response2 = await engine.setSources(
        trustedSources: {trusted2, duplicate},
        excludedSources: {excluded2, duplicate},
      );

      expect(response2, isA<SetSourcesRequestFailed>());
      expect(
        (response2 as SetSourcesRequestFailed).duplicateSources,
        equals({duplicate}),
      );
      expect(
        expectEvent<TrustedSourcesListRequestSucceeded>(
          await engine.getTrustedSourcesList(),
        ).sources,
        equals({trusted}),
      );
      expect(
        expectEvent<ExcludedSourcesListRequestSucceeded>(
          await engine.getExcludedSourcesList(),
        ).excludedSources,
        equals({excluded}),
      );

      final response3 = await engine.setSources(
        trustedSources: {trusted, trusted3},
        excludedSources: {},
      );
      expect(response3, isA<SetSourcesRequestSucceeded>());
      expect(
        (response3 as SetSourcesRequestSucceeded).trustedSources,
        equals({trusted, trusted3}),
      );
      expect(response3.excludedSources, equals(<Source>{}));
      expect(
        expectEvent<TrustedSourcesListRequestSucceeded>(
          await engine.getTrustedSourcesList(),
        ).sources,
        equals({trusted, trusted3}),
      );
      expect(
        expectEvent<ExcludedSourcesListRequestSucceeded>(
          await engine.getExcludedSourcesList(),
        ).excludedSources,
        equals(<Source>{}),
      );
    });

    test('trusted and excluded sources for the same domain can\'t co-exist',
        () async {
      final response1 =
          await engine.addSourceToTrustedList(Source('example.com'));
      final response2 =
          await engine.addSourceToExcludedList(Source('example.com'));
      expect(response1, isA<AddTrustedSourceRequestSucceeded>());
      expect(response2, isA<AddExcludedSourceRequestSucceeded>());

      expect(
        expectEvent<ExcludedSourcesListRequestSucceeded>(
          await engine.getExcludedSourcesList(),
        ).excludedSources,
        equals({Source('example.com')}),
      );

      expect(
        expectEvent<TrustedSourcesListRequestSucceeded>(
          await engine.getTrustedSourcesList(),
        ).sources,
        isEmpty,
      );
    });

    test('adding sources preferences triggers updates to stacks', () async {
      //needed due to the out of sync initial update
      while (server.requestCount < 2) {
        await Future<void>.delayed(const Duration(milliseconds: 10));
      }
      var lastCount = server.requestCount;
      await engine.addSourceToTrustedList(trusted);
      expect(server.requestCount, greaterThan(lastCount));

      lastCount = server.requestCount;
      await engine.addSourceToTrustedList(trusted);
      expect(server.requestCount, equals(lastCount));

      lastCount = server.requestCount;
      await engine.addSourceToExcludedList(excluded);
      expect(server.requestCount, greaterThan(lastCount));

      lastCount = server.requestCount;
      await engine.addSourceToExcludedList(excluded);
      expect(server.requestCount, equals(lastCount));

      lastCount = server.requestCount;
      await engine
          .setSources(trustedSources: {trusted}, excludedSources: {excluded});
      expect(server.requestCount, equals(lastCount));

      lastCount = server.requestCount;
      await engine.setSources(trustedSources: {}, excludedSources: {});
      expect(server.requestCount, greaterThan(lastCount));
    });
  });

  test('DiscoveryEngine source preferences persists between engine instances',
      () async {
    final server = await LocalNewsApiServer.start();
    final data = await setupTestEngineData(useInMemoryDb: false);
    var engine = await initEngine(data, server.port);

    await engine.setSources(
      trustedSources: {Source('trusted.example'), Source('trusted2.example')},
      excludedSources: {Source('excluded.example')},
    );

    await engine.dispose();
    engine = await initEngine(data, server.port);

    expect(
      expectEvent<TrustedSourcesListRequestSucceeded>(
        await engine.getTrustedSourcesList(),
      ).sources,
      equals({Source('trusted.example'), Source('trusted2.example')}),
    );

    expect(
      expectEvent<ExcludedSourcesListRequestSucceeded>(
        await engine.getExcludedSourcesList(),
      ).excludedSources,
      equals({Source('excluded.example')}),
    );

    await server.close();
    await Directory(data.applicationDirectoryPath).delete(recursive: true);
  });
}
