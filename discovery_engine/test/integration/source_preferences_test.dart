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

  group('DiscoveryEngine source preferences', () {
    late LocalNewsApiServer server;
    late TestEngineData data;
    late DiscoveryEngine engine;

    final exclude = Source('example.com');
    final trusted = Source('xayn.com');

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
        await engine.addSourceToExcludedList(exclude),
      );

      final listResponse = expectEvent<ExcludedSourcesListRequestSucceeded>(
        await engine.getExcludedSourcesList(),
      );
      expect(listResponse.excludedSources, equals({exclude}));

      final nextBatchResponse = expectEvent<NextFeedBatchRequestFailed>(
        await engine.requestNextFeedBatch(),
      );
      expect(nextBatchResponse.reason, FeedFailureReason.noNewsForMarket);

      expect(server.lastCapturedRequest, isNotNull);
      server.lastCapturedRequest!
          .expectJsonQueryParams({'not_sources': exclude.toString()});
    });

    test('removeSourceFromExcludedList removes the added excluded source',
        () async {
      expectEvent<AddExcludedSourceRequestSucceeded>(
        await engine.addSourceToExcludedList(exclude),
      );

      var listResponse = expectEvent<ExcludedSourcesListRequestSucceeded>(
        await engine.getExcludedSourcesList(),
      );
      expect(listResponse.excludedSources, equals({exclude}));

      expectEvent<RemoveExcludedSourceRequestSucceeded>(
        await engine.removeSourceFromExcludedList(exclude),
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
        await engine.addSourceToExcludedList(Source('example.org')),
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
        trustedSources: {Source('trusted1.local')},
        excludedSources: {Source('excluded1.local')},
      );

      expect(response1, isA<SetSourcesRequestSucceeded>());
      expect(
        expectEvent<TrustedSourcesListRequestSucceeded>(
          await engine.getTrustedSourcesList(),
        ).sources,
        equals({Source('trusted1.local')}),
      );
      expect(
        expectEvent<ExcludedSourcesListRequestSucceeded>(
          await engine.getExcludedSourcesList(),
        ).excludedSources,
        equals({Source('excluded1.local')}),
      );

      final response2 = await engine.setSources(
        trustedSources: {Source('trusted2.local'), Source('duplicate.local')},
        excludedSources: {
          Source('excluded2.local'),
          Source('duplicate.local'),
        },
      );

      expect(response2, isA<SetSourcesRequestFailed>());
      expect(
        (response2 as SetSourcesRequestFailed).duplicateSources,
        equals({Source('duplicate.local')}),
      );
      expect(
        expectEvent<TrustedSourcesListRequestSucceeded>(
          await engine.getTrustedSourcesList(),
        ).sources,
        equals({Source('trusted1.local')}),
      );
      expect(
        expectEvent<ExcludedSourcesListRequestSucceeded>(
          await engine.getExcludedSourcesList(),
        ).excludedSources,
        equals({Source('excluded1.local')}),
      );

      final response3 = await engine.setSources(
        trustedSources: {Source('trusted1.local'), Source('trusted3.local')},
        excludedSources: {},
      );
      expect(response3, isA<SetSourcesRequestSucceeded>());
      expect(
        (response3 as SetSourcesRequestSucceeded).trustedSources,
        equals({Source('trusted1.local'), Source('trusted3.local')}),
      );
      expect(response3.excludedSources, equals(<Source>{}));
      expect(
        expectEvent<TrustedSourcesListRequestSucceeded>(
          await engine.getTrustedSourcesList(),
        ).sources,
        equals({Source('trusted1.local'), Source('trusted3.local')}),
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

    //TODO[PMK] check if updates stacks is called
  });
}
