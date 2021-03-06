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

import 'package:hive/hive.dart' show Hive;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/api/api.dart' hide Document;
import 'package:xayn_discovery_engine/src/domain/engine/mock_engine.dart'
    show MockEngine;
import 'package:xayn_discovery_engine/src/domain/event_handler.dart'
    show EventConfig, EventHandler;
import 'package:xayn_discovery_engine/src/domain/feed_manager.dart'
    show FeedManager;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show mockedAvailableSources, Source;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart'
    show HiveActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_document_repo.dart'
    show HiveDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_engine_state_repo.dart'
    show HiveEngineStateRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_source_preference_repo.dart'
    show HiveSourcePreferenceRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_source_reacted_repo.dart';

import 'discovery_engine/utils/utils.dart' show mockDocuments, mockNewsResource;
import 'logging.dart' show setupLogging;

Future<void> main() async {
  setupLogging();

  late HiveDocumentRepository docRepo;
  late HiveActiveDocumentDataRepository activeRepo;
  late HiveEngineStateRepository engineStateRepo;
  late HiveSourceReactedRepository sourceReactedRepo;
  late HiveSourcePreferenceRepository sourcePreferenceRepo;
  late FeedManager mgr;

  final engine = MockEngine()..feedDocuments = mockDocuments(StackId(), false);
  final config = EventConfig(maxFeedDocs: 5, maxSearchDocs: 20);

  EventHandler.registerHiveAdapters();

  group('FeedManager', () {
    late ActiveDocumentData data;
    late Document doc2, doc3;
    late DocumentId id2, id3;
    final id = DocumentId();

    setUp(() async {
      final dir = Directory.systemTemp.createTempSync('FeedManager');
      await EventHandler.initDatabase(dir.path);

      docRepo = HiveDocumentRepository();
      activeRepo = HiveActiveDocumentDataRepository();
      engineStateRepo = HiveEngineStateRepository();
      sourceReactedRepo = HiveSourceReactedRepository();
      sourcePreferenceRepo = HiveSourcePreferenceRepository();
      mgr = FeedManager(
        engine,
        config,
        docRepo,
        activeRepo,
        engineStateRepo,
        sourceReactedRepo,
        sourcePreferenceRepo,
        mockedAvailableSources,
      );

      data = ActiveDocumentData(Embedding.fromList([44]));
      final stackId = StackId();
      doc2 = Document(
        documentId: DocumentId(),
        stackId: stackId,
        batchIndex: 2,
        resource: mockNewsResource,
        isActive: true,
      );
      doc3 = Document(
        documentId: DocumentId(),
        stackId: stackId,
        batchIndex: 3,
        resource: mockNewsResource,
        isActive: false,
      );
      id2 = doc2.documentId;
      id3 = doc3.documentId;

      // doc2 is active & changed, doc3 is neither
      await docRepo.updateMany([doc2, doc3]);
      await activeRepo.update(id2, data);

      engine.resetCallCounter();
    });

    tearDown(() async {
      await Hive.deleteFromDisk();
    });

    test('deactivate documents', () async {
      final evt = await mgr.deactivateDocuments({id2, id3, id});
      expect(evt is ClientEventSucceeded, isTrue);

      // id2 should be removed from active and changed repos
      expect(activeRepo.box, isEmpty);

      // id2 should now be deactivated, id3 still inactive
      expect(docRepo.box, hasLength(2));
      final docs = await docRepo.fetchByIds({id2, id3});
      expect(docs, hasLength(2));
      expect(docs[0].isActive, isFalse);
      expect(docs[1].isActive, isFalse);
    });

    test('get next feed batch', () async {
      final evt = await mgr.nextFeedBatch();
      expect(engine.getCallCount('getFeedDocuments'), equals(1));
      final docs =
          evt.whenOrNull(nextFeedBatchRequestSucceeded: (docs) => docs);

      // check returned ids match those of mock engine
      expect(docs, isNotNull);
      expect(
        docs!.map((doc) => doc.documentId),
        unorderedEquals(
          engine.feedDocuments
              .map<DocumentId>((doc) => doc.document.documentId),
        ),
      );

      // check repositories are updated
      expect(docRepo.box, hasLength(4));
      expect(
        docRepo.box.values,
        containsAll(engine.feedDocuments.map<Document>((doc) => doc.document)),
      );
      expect(activeRepo.box, hasLength(3));
      expect(
        activeRepo.box.values,
        containsAll(
          engine.feedDocuments.map<ActiveDocumentData>((doc) => doc.data),
        ),
      );

      // serialize should be called and state saved
      expect(engine.getCallCount('serialize'), equals(1));
      expect(engineStateRepo.box.isNotEmpty, isTrue);
    });

    test('restore feed', () async {
      final earlier = DateTime.utc(1969, 7, 20);
      final later = DateTime.utc(1989, 11, 9);
      await docRepo.update(doc2..timestamp = earlier);
      await docRepo.update(doc3..timestamp = later);
      await docRepo.update(engine.feedDocuments[0].document..timestamp = later);
      await docRepo
          .update(engine.feedDocuments[1].document..timestamp = earlier);

      expect(docRepo.box, hasLength(4));

      final evt = await mgr.restoreFeed();
      final feed = evt.whenOrNull(restoreFeedSucceeded: (docs) => docs);

      expect(feed, isNotNull);
      expect(feed, hasLength(3));
      // doc1, doc2 have the earlier timestamp
      expect(
        feed![0].documentId,
        equals(engine.feedDocuments[1].document.documentId),
      );
      expect(feed[0].batchIndex, equals(1));
      expect(feed[1].documentId, equals(doc2.documentId));
      expect(feed[1].batchIndex, equals(2));
      // doc0 has the later timestamp
      expect(
        feed[2].documentId,
        equals(engine.feedDocuments[0].document.documentId),
      );
      expect(feed[2].batchIndex, equals(0));
      // doc3 is excluded since it is inactive
    });
  });

  group('Source preferences', () {
    setUp(() async {
      final dir = Directory.systemTemp.createTempSync('SourcePreferences');
      await EventHandler.initDatabase(dir.path);

      docRepo = HiveDocumentRepository();
      activeRepo = HiveActiveDocumentDataRepository();
      engineStateRepo = HiveEngineStateRepository();
      sourceReactedRepo = HiveSourceReactedRepository();
      sourcePreferenceRepo = HiveSourcePreferenceRepository();
      mgr = FeedManager(
        engine,
        config,
        docRepo,
        activeRepo,
        engineStateRepo,
        sourceReactedRepo,
        sourcePreferenceRepo,
        mockedAvailableSources,
      );
      engine.resetCallCounter();
    });

    tearDown(() async {
      await Hive.deleteFromDisk();
    });

    test('setSources', () async {
      final response1 = await mgr.setSources(
        {Source('trusted1.local')},
        {Source('excluded1.local')},
      );

      expect(response1, isA<SetSourcesRequestSucceeded>());
      expect(
        await sourcePreferenceRepo.getTrusted(),
        equals({Source('trusted1.local')}),
      );
      expect(
        await sourcePreferenceRepo.getExcluded(),
        equals({Source('excluded1.local')}),
      );

      final response2 = await mgr.setSources({
        Source('trusted2.local'),
        Source('duplicate.local')
      }, {
        Source('excluded2.local'),
        Source('duplicate.local'),
      });

      expect(response2, isA<SetSourcesRequestFailed>());
      expect(
        (response2 as SetSourcesRequestFailed).duplicateSources,
        equals({Source('duplicate.local')}),
      );
      expect(
        await sourcePreferenceRepo.getTrusted(),
        equals({Source('trusted1.local')}),
      );
      expect(
        await sourcePreferenceRepo.getExcluded(),
        equals({Source('excluded1.local')}),
      );

      final response3 = await mgr.setSources(
        {Source('trusted1.local'), Source('trusted3.local')},
        {},
      );
      expect(response3, isA<SetSourcesRequestSucceeded>());
      expect(
        (response3 as SetSourcesRequestSucceeded).trustedSources,
        equals({Source('trusted1.local'), Source('trusted3.local')}),
      );
      expect(response3.excludedSources, equals(<Source>{}));
      expect(
        await sourcePreferenceRepo.getTrusted(),
        equals({Source('trusted1.local'), Source('trusted3.local')}),
      );
      expect(
        await sourcePreferenceRepo.getExcluded(),
        equals(<Source>{}),
      );
    });

    test('setSources should call the engine methods only when sources change',
        () async {
      final response1 = await mgr.setSources(
        {Source('trusted1.local'), Source('trusted2.local')},
        {Source('excluded1.local')},
      );
      expect(response1, isA<SetSourcesRequestSucceeded>());
      expect(engine.getCallCount('setTrustedSources'), equals(1));
      expect(engine.getCallCount('setExcludedSources'), equals(1));

      engine.resetCallCounter();

      final response2 = await mgr.setSources(
        {Source('trusted2.local'), Source('trusted3.local')},
        {Source('excluded1.local')},
      );
      expect(response2, isA<SetSourcesRequestSucceeded>());
      expect(engine.getCallCount('setTrustedSources'), equals(1));
      expect(engine.getCallCount('setExcludedSources'), equals(0));

      engine.resetCallCounter();

      final response3 = await mgr.setSources(
        // here we've changed the order
        {Source('trusted3.local'), Source('trusted2.local')},
        {Source('excluded1.local')},
      );
      expect(response3, isA<SetSourcesRequestSucceeded>());
      expect(engine.getCallCount('setTrustedSources'), equals(0));
      expect(engine.getCallCount('setExcludedSources'), equals(0));
    });

    test('addExcludedSource', () async {
      final response1 = await mgr.addExcludedSource(Source('test1.local'));
      final response2 = await mgr.addExcludedSource(Source('test2.local'));

      expect(response1, isA<AddExcludedSourceRequestSucceeded>());
      expect(response2, isA<AddExcludedSourceRequestSucceeded>());

      final content = await mgr.getExcludedSourcesList();
      expect(content, isA<ExcludedSourcesListRequestSucceeded>());
      expect(
        (content as ExcludedSourcesListRequestSucceeded).excludedSources,
        equals({Source('test1.local'), Source('test2.local')}),
      );
    });

    test('addTrustedSource', () async {
      final response1 = await mgr.addTrustedSource(Source('test3.local'));
      final response2 = await mgr.addTrustedSource(Source('test4.local'));

      expect(response1, isA<AddTrustedSourceRequestSucceeded>());
      expect(response2, isA<AddTrustedSourceRequestSucceeded>());

      final content = await mgr.getTrustedSourcesList();
      expect(content, isA<TrustedSourcesListRequestSucceeded>());
      expect(
        (content as TrustedSourcesListRequestSucceeded).sources,
        equals({Source('test3.local'), Source('test4.local')}),
      );
    });

    test('removeExcludedSource', () async {
      await mgr.addExcludedSource(Source('test1.local'));
      await mgr.addExcludedSource(Source('test2.local'));

      final response = await mgr.removeExcludedSource(Source('test1.local'));
      expect(response, isA<RemoveExcludedSourceRequestSucceeded>());

      final content = await mgr.getExcludedSourcesList();
      expect(content, isA<ExcludedSourcesListRequestSucceeded>());
      expect(
        (content as ExcludedSourcesListRequestSucceeded).excludedSources,
        equals({Source('test2.local')}),
      );
    });

    test('removeTrustedSource', () async {
      await mgr.addTrustedSource(Source('test5.local'));
      await mgr.addTrustedSource(Source('test6.local'));

      final response = await mgr.removeTrustedSource(Source('test5.local'));
      expect(response, isA<RemoveTrustedSourceRequestSucceeded>());

      final content = await mgr.getTrustedSourcesList();
      expect(content, isA<TrustedSourcesListRequestSucceeded>());
      expect(
        (content as TrustedSourcesListRequestSucceeded).sources,
        equals({Source('test6.local')}),
      );
    });

    test('getting source lists', () async {
      await mgr.addTrustedSource(Source('test7.local'));
      await mgr.addExcludedSource(Source('example.com'));
      await mgr.addExcludedSource(Source('test8.local'));
      await mgr.addTrustedSource(Source('test9.local'));

      var response = await mgr.getExcludedSourcesList();
      expect(response, isA<ExcludedSourcesListRequestSucceeded>());
      expect(
        (response as ExcludedSourcesListRequestSucceeded).excludedSources,
        equals({
          Source('example.com'),
          Source('test8.local'),
        }),
      );

      response = await mgr.getTrustedSourcesList();
      expect(response, isA<TrustedSourcesListRequestSucceeded>());
      expect(
        (response as TrustedSourcesListRequestSucceeded).sources,
        equals({
          Source('test7.local'),
          Source('test9.local'),
        }),
      );
    });

    test('trusted and excluded sources for the same domain can\'t co-exist',
        () async {
      final response1 = await mgr.addTrustedSource(Source('example.com'));
      final response2 = await mgr.addExcludedSource(Source('example.com'));
      expect(response1, isA<AddTrustedSourceRequestSucceeded>());
      expect(response2, isA<AddExcludedSourceRequestSucceeded>());

      var response = await mgr.getExcludedSourcesList();
      expect(response, isA<ExcludedSourcesListRequestSucceeded>());
      expect(
        (response as ExcludedSourcesListRequestSucceeded).excludedSources,
        equals({
          Source('example.com'),
        }),
      );
      expect(
        engine.excludedSources,
        equals({Source('example.com')}),
      );

      response = await mgr.getTrustedSourcesList();
      expect(response, isA<TrustedSourcesListRequestSucceeded>());
      expect(
        (response as TrustedSourcesListRequestSucceeded).sources,
        isEmpty,
      );
      expect(engine.trustedSources, equals(<Source>{}));
    });
  });
}
