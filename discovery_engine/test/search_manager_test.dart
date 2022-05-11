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
import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart' show Hive;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart'
    show
        ClientEventSucceeded,
        NextSearchBatchRequestFailed,
        NextSearchBatchRequestSucceeded,
        RestoreSearchFailed,
        RestoreSearchSucceeded,
        SearchFailureReason,
        SearchRequestSucceeded,
        SearchTermRequestFailed,
        SearchTermRequestSucceeded,
        TrendingTopicsRequestFailed,
        TrendingTopicsRequestSucceeded;
import 'package:xayn_discovery_engine/src/api/models/active_search.dart'
    show ActiveSearchApiConversion;
import 'package:xayn_discovery_engine/src/api/models/document.dart'
    show DocumentApiConversion;
import 'package:xayn_discovery_engine/src/domain/engine/mock_engine.dart'
    show MockEngine, mockTrendingTopic;
import 'package:xayn_discovery_engine/src/domain/event_handler.dart'
    show EventConfig, EventHandler;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    show ActiveSearch, SearchBy;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/trending_topic.dart'
    show TrendingTopic;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/search_manager.dart'
    show SearchManager;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart'
    show HiveActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_search_repo.dart'
    show HiveActiveSearchRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_document_repo.dart'
    show HiveDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_engine_state_repo.dart'
    show HiveEngineStateRepository;

import 'discovery_engine/utils/utils.dart'
    show mockActiveSearch, mockNewsResource;
import 'logging.dart' show setupLogging;

Future<void> main() async {
  group('SearchManager', () {
    setupLogging();

    late HiveDocumentRepository docRepo;
    late HiveActiveSearchRepository searchRepo;
    late HiveActiveDocumentDataRepository activeRepo;
    late HiveEngineStateRepository engineStateRepo;
    late SearchManager mgr;

    final engine = MockEngine();
    final config = EventConfig(maxFeedDocs: 5, maxSearchDocs: 20);
    final data = ActiveDocumentData(Embedding.fromList([44]));
    final stackId = StackId.fromBytes(Uint8List.fromList(List.filled(16, 0)));
    var doc1 = Document(
      documentId: DocumentId(),
      stackId: stackId,
      batchIndex: 1,
      resource: mockNewsResource,
    );
    var doc2 = Document(
      documentId: DocumentId(),
      stackId: stackId,
      batchIndex: 2,
      resource: mockNewsResource,
    );

    setUpAll(() async {
      EventHandler.registerHiveAdapters();
    });

    setUp(() async {
      final dir = Directory.systemTemp.createTempSync('SearchManager');
      await EventHandler.initDatabase(dir.path);

      docRepo = HiveDocumentRepository();
      searchRepo = HiveActiveSearchRepository();
      activeRepo = HiveActiveDocumentDataRepository();
      engineStateRepo = HiveEngineStateRepository();
      mgr = SearchManager(
        engine,
        config,
        searchRepo,
        docRepo,
        activeRepo,
        engineStateRepo,
      );

      doc1 = doc1
        ..isSearched = true
        ..isActive = true;
      doc2 = doc2
        ..isSearched = true
        ..isActive = true;

      await searchRepo.save(mockActiveSearch);
      await docRepo.updateMany([doc1, doc2]);
      await activeRepo.update(doc1.documentId, data);
      await activeRepo.update(doc2.documentId, data);
    });

    tearDown(() async {
      await Hive.deleteFromDisk();
      engine.resetCallCounter();
    });

    group('searchRequested', () {
      test(
          'given a query term a proper active search object is persisted, '
          'and new document and active data entries are added to the database',
          () async {
        doc1 = doc1..isSearched = false;
        doc2 = doc2..isSearched = false;
        await docRepo.updateMany([doc1, doc2]);

        final newSearch = ActiveSearch(
          searchTerm: 'example query',
          requestedPageNb: 1,
          pageSize: config.maxSearchDocs,
          searchBy: SearchBy.query,
        );

        final response =
            await mgr.searchRequested('example query', SearchBy.query);

        expect(searchRepo.getCurrent(), completion(equals(newSearch)));
        expect(response, isA<SearchRequestSucceeded>());
        expect(
          (response as SearchRequestSucceeded).search,
          equals(newSearch.toApiRepr()),
        );
        expect(response.items.length, equals(2));

        final savedDocs = response.items
            // lets look for the docs in the document box
            .map((doc) => docRepo.box.get('${doc.documentId}'))
            .map((doc) => doc!.toApiRepr())
            .toList();

        expect(response.items, equals(savedDocs));
        // we have 2 more documents in the database
        expect(docRepo.box.length, equals(4));
        // we have 2 document data entries in active box under proper ids
        expect(
          activeRepo.box.get('${response.items.first.documentId}'),
          isNotNull,
        );
        expect(
          activeRepo.box.get('${response.items.last.documentId}'),
          isNotNull,
        );
        expect(engine.getCallCount('activeSearch'), equals(1));
        expect(engine.getCallCount('serialize'), equals(1));
      });
    });

    group('nextSearchBatchRequested', () {
      test(
          'when there is no active search stored it should return '
          '"NextSearchBatchRequestFailed" event with "noActiveSearch" reason',
          () async {
        // lets clear the repo
        await searchRepo.clear();

        final response = await mgr.nextSearchBatchRequested();

        expect(response, isA<NextSearchBatchRequestFailed>());
        expect(
          (response as NextSearchBatchRequestFailed).reason,
          SearchFailureReason.noActiveSearch,
        );
      });

      test('active search "requestedPageNb" attribute should be incremented ',
          () async {
        final response = await mgr.nextSearchBatchRequested();

        final updateSearch =
            (response as NextSearchBatchRequestSucceeded).search;

        expect(response, isA<NextSearchBatchRequestSucceeded>());
        final current = await searchRepo.getCurrent();
        expect(current?.toApiRepr(), equals(updateSearch));
        expect(response.items.length, equals(2));

        final savedDocs = response.items
            // lets look for the docs in the document box
            .map((doc) => docRepo.box.get('${doc.documentId}'))
            .map((doc) => doc!.toApiRepr())
            .toList();

        expect(response.items, equals(savedDocs));
        // we have 2 more documents in the database
        expect(docRepo.box.length, equals(4));
        // we have 2 document data entries in active box under proper ids
        expect(
          activeRepo.box.get('${response.items.first.documentId}'),
          isNotNull,
        );
        expect(
          activeRepo.box.get('${response.items.last.documentId}'),
          isNotNull,
        );
        expect(engine.getCallCount('activeSearch'), equals(1));
        expect(engine.getCallCount('serialize'), equals(1));
      });
    });

    group('restoreSearchRequested', () {
      test(
          'when there is no active search stored it should return '
          '"RestoreSearchFailed" event with "noActiveSearch" reason', () async {
        // lets clear the repo
        await searchRepo.clear();

        final response = await mgr.restoreSearchRequested();

        expect(response, isA<RestoreSearchFailed>());
        expect(
          (response as RestoreSearchFailed).reason,
          SearchFailureReason.noActiveSearch,
        );
      });

      test(
          'when there are no search related documents it should return '
          '"RestoreSearchFailed" event with "noResultsAvailable" reason',
          () async {
        doc1 = doc1..isActive = false;
        doc2 = doc2..isSearched = false;
        await docRepo.updateMany([doc1, doc2]);

        final response = await mgr.restoreSearchRequested();

        expect(response, isA<RestoreSearchFailed>());
        expect(
          (response as RestoreSearchFailed).reason,
          SearchFailureReason.noResultsAvailable,
        );
      });

      test(
          'when there is active search and related documents to restore '
          'it should return "RestoreSearchSucceeded" event with them',
          () async {
        final response = await mgr.restoreSearchRequested();

        expect(response, isA<RestoreSearchSucceeded>());
        expect(
          response,
          equals(
            RestoreSearchSucceeded(
              mockActiveSearch.toApiRepr(),
              [doc1.toApiRepr(), doc2.toApiRepr()],
            ),
          ),
        );
        expect(engine.getCallCount('activeSearch'), equals(0));
        expect(engine.getCallCount('serialize'), equals(0));
      });
    });

    group('searchTermRequested', () {
      test(
          'when there is no active search stored it should return '
          '"SearchTermRequestFailed" event with "noActiveSearch" reason',
          () async {
        // lets clear the repo
        await searchRepo.clear();

        final response = await mgr.searchTermRequested();

        expect(response, isA<SearchTermRequestFailed>());
        expect(
          (response as SearchTermRequestFailed).reason,
          SearchFailureReason.noActiveSearch,
        );
      });

      test(
          'if active search is available it should return '
          '"SearchTermRequestSucceeded" event with the current search term',
          () async {
        final response = await mgr.searchTermRequested();

        expect(response, isA<SearchTermRequestSucceeded>());
        expect(
          (response as SearchTermRequestSucceeded).searchTerm,
          mockActiveSearch.searchTerm,
        );
      });
    });

    group('trendingTopicsRequested', () {
      test(
          'when there are no topics found it should return '
          '"TrendingTopicsRequestFailed" event with "noResultsAvailable" reason',
          () async {
        final engine = _NoTrendingTopicsMockEngine();
        final mgr = SearchManager(
          engine,
          config,
          searchRepo,
          docRepo,
          activeRepo,
          engineStateRepo,
        );
        final response = await mgr.trendingTopicsRequested();

        expect(response, isA<TrendingTopicsRequestFailed>());
        expect(
          (response as TrendingTopicsRequestFailed).reason,
          SearchFailureReason.noResultsAvailable,
        );
      });

      test(
          'if active search is available it should return '
          '"SearchTermRequestSucceeded" event with the current search term',
          () async {
        final response = await mgr.trendingTopicsRequested();

        expect(response, isA<TrendingTopicsRequestSucceeded>());
        expect(
          (response as TrendingTopicsRequestSucceeded).topics,
          [mockTrendingTopic],
        );
      });
    });

    group('searchClosed', () {
      test(
          'when there are no search documents it should return '
          '"ClientEventSucceeded" event in response and clear only '
          'the active search repo', () async {
        doc1 = doc1..isSearched = false;
        doc2 = doc2..isSearched = false;
        await docRepo.updateMany([doc1, doc2]);

        final response = await mgr.searchClosed();

        expect(response, isA<ClientEventSucceeded>());
        expect(searchRepo.box.isEmpty, isTrue);
        expect(activeRepo.box.length, equals(2));
        expect(docRepo.box.length, equals(2));
        expect(doc1.isActive, isTrue);
        expect(doc2.isActive, isTrue);
      });

      test(
          'it should clear active search repo, active and changed documents '
          'from searched ids, and leave only user reacted documents '
          'in the documents repo', () async {
        doc1 = doc1..isSearched = false;
        doc2 = doc2..userReaction = UserReaction.positive;
        final doc3 = Document(
          stackId: stackId,
          resource: mockNewsResource,
          batchIndex: 3,
          documentId: DocumentId(),
          userReaction: UserReaction.negative,
          isSearched: true,
        );
        final doc4 = Document(
          stackId: stackId,
          resource: mockNewsResource,
          batchIndex: 4,
          documentId: DocumentId(),
          isSearched: true,
        );
        await activeRepo.update(doc3.documentId, data);
        await activeRepo.update(doc4.documentId, data);
        await docRepo.updateMany([doc1, doc2, doc3, doc4]);

        final response = await mgr.searchClosed();

        expect(response, isA<ClientEventSucceeded>());
        expect(searchRepo.box.isEmpty, isTrue);
        // only doc1 should be left in the active data and changed doc boxes
        expect(activeRepo.box.length, equals(1));
        expect(activeRepo.box.get('${doc1.documentId}'), equals(data));
        expect(docRepo.box.length, equals(3));
        // doc1 wasn't searched so it should stay active
        expect(doc1.isActive, isTrue);
        // doc2 and doc3 were searched but non-neutral so they should be kept
        // in the satabase, but not be active anymore
        expect(doc2.isActive, isFalse);
        expect(doc3.isActive, isFalse);
        // doc4 was searched but neutral so it should be removed from db
        expect(docRepo.box.get('${doc4.documentId}'), isNull);
      });
    });
  });
}

class _NoTrendingTopicsMockEngine extends MockEngine {
  @override
  Future<List<TrendingTopic>> getTrendingTopics() async => [];
}
