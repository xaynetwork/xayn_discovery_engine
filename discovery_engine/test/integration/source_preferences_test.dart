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
        NextFeedBatchRequestSucceeded,
        cfgFeatureStorage;
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart';
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show Source;

import '../logging.dart' show setupLogging;
import 'utils/helpers.dart'
    show TestEngineData, initEngine, setupTestEngineData, expectEvent;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

void main() {
  setupLogging();

  final excluded = Source('example.com');
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
      final response1 = await engine.overrideSources(
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

      final response2 = await engine.overrideSources(
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

      final response3 = await engine.overrideSources(
        trustedSources: {trusted, trusted3},
        excludedSources: {},
      );
      expect(response3, isA<SetSourcesRequestSucceeded>());
      expect(
        (response3 as SetSourcesRequestSucceeded).trustedSources,
        equals({trusted, trusted3}),
      );
      expect(response3.excludedSources, isEmpty);
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
        isEmpty,
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

    test(
        'adding sources preferences triggers updates to stacks iff sources changed',
        () async {
      //needed due to the out of sync initial update
      while (server.requestCount < 2) {
        await Future<void>.delayed(const Duration(milliseconds: 10));
      }
      var lastCount = server.requestCount;
      await engine.addSourceToTrustedList(trusted);
      expect(server.requestCount, greaterThan(lastCount));

      // the old code does sometimes run updates even if not needed
      if (cfgFeatureStorage) {
        lastCount = server.requestCount;
        await engine.addSourceToTrustedList(trusted);
        expect(server.requestCount, equals(lastCount));
      }

      lastCount = server.requestCount;
      await engine.addSourceToExcludedList(excluded);
      expect(server.requestCount, greaterThan(lastCount));

      if (cfgFeatureStorage) {
        lastCount = server.requestCount;
        await engine.addSourceToExcludedList(excluded);
        expect(server.requestCount, equals(lastCount));

        lastCount = server.requestCount;
        await engine.overrideSources(
          trustedSources: {trusted},
          excludedSources: {excluded},
        );
        expect(server.requestCount, equals(lastCount));
      }

      lastCount = server.requestCount;
      await engine.overrideSources(trustedSources: {}, excludedSources: {});
      expect(server.requestCount, greaterThan(lastCount));
    });
  });

  group('DiscoveryEngine source preferences with persistent storage', () {
    late LocalNewsApiServer server;
    late TestEngineData data;
    late DiscoveryEngine engine;

    setUp(() async {
      server = await LocalNewsApiServer.start();
      data = await setupTestEngineData(useEphemeralDb: false);
      engine = await initEngine(data, server.port);
    });

    tearDown(() async {
      await engine.dispose();
      await server.close();
      await Directory(data.applicationDirectoryPath).delete(recursive: true);
    });

    test('setSources persists over engine instances', () async {
      expectEvent<SetSourcesRequestSucceeded>(
        await engine.overrideSources(
          trustedSources: {trusted, trusted2},
          excludedSources: {excluded},
        ),
      );

      await engine.dispose();
      engine = await initEngine(data, server.port);

      expect(
        expectEvent<TrustedSourcesListRequestSucceeded>(
          await engine.getTrustedSourcesList(),
        ).sources,
        equals({trusted, trusted2}),
      );

      expect(
        expectEvent<ExcludedSourcesListRequestSucceeded>(
          await engine.getExcludedSourcesList(),
        ).excludedSources,
        equals({excluded}),
      );
    });

    test('add/remove trusted/excluded sources persist over engine instances',
        () async {
      expectEvent<AddExcludedSourceRequestSucceeded>(
        await engine.addSourceToExcludedList(excluded),
      );
      expectEvent<AddExcludedSourceRequestSucceeded>(
        await engine.addSourceToExcludedList(excluded2),
      );
      expectEvent<AddTrustedSourceRequestSucceeded>(
        await engine.addSourceToTrustedList(trusted),
      );
      expectEvent<AddTrustedSourceRequestSucceeded>(
        await engine.addSourceToTrustedList(trusted2),
      );
      expectEvent<AddTrustedSourceRequestSucceeded>(
        await engine.addSourceToTrustedList(trusted3),
      );
      expectEvent<RemoveExcludedSourceRequestSucceeded>(
        await engine.removeSourceFromExcludedList(excluded2),
      );
      expectEvent<RemoveTrustedSourceRequestSucceeded>(
        await engine.removeSourceFromTrustedList(trusted2),
      );

      await engine.dispose();
      engine = await initEngine(data, server.port);

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
        equals({excluded}),
      );
    });
  });
}
