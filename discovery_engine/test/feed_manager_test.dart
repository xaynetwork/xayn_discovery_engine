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

import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart';
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart'
    show ClientEventSucceeded, ExcludedSourcesListRequestSucceeded;
import 'package:xayn_discovery_engine/src/domain/engine/mock_engine.dart'
    show MockEngine;
import 'package:xayn_discovery_engine/src/domain/event_handler.dart'
    show EventConfig;
import 'package:xayn_discovery_engine/src/domain/feed_manager.dart'
    show FeedManager;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, DocumentAdapter;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show
        activeDocumentDataBox,
        changedDocumentIdBox,
        documentBox,
        engineStateBox,
        excludedSourcesBox;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart'
    show HiveActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_changed_document_repo.dart'
    show HiveChangedDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_document_repo.dart'
    show HiveDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_engine_state_repo.dart'
    show HiveEngineStateRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_excluded_sources_repo.dart'
    show HiveExcludedSourcesRepository;

import 'discovery_engine/utils/utils.dart';
import 'logging.dart' show setupLogging;

Future<void> main() async {
  setupLogging();

  Hive.registerAdapter(DocumentAdapter());

  final docBox = await Hive.openBox<Document>(documentBox, bytes: Uint8List(0));
  final activeBox = await Hive.openBox<ActiveDocumentData>(
    activeDocumentDataBox,
    bytes: Uint8List(0),
  );
  final changedBox =
      await Hive.openBox<Uint8List>(changedDocumentIdBox, bytes: Uint8List(0));
  final stateBox =
      await Hive.openBox<Uint8List>(engineStateBox, bytes: Uint8List(0));
  final excludedBox =
      await Hive.openBox<Set<String>>(excludedSourcesBox, bytes: Uint8List(0));

  final engine = MockEngine();
  final config = EventConfig(maxFeedDocs: 5, maxSearchDocs: 20);
  final docRepo = HiveDocumentRepository();
  final activeRepo = HiveActiveDocumentDataRepository();
  final changedRepo = HiveChangedDocumentRepository();
  final engineStateRepo = HiveEngineStateRepository();
  final excludedSourcesRepo = HiveExcludedSourcesRepository();

  final mgr = FeedManager(
    engine,
    config,
    docRepo,
    activeRepo,
    changedRepo,
    engineStateRepo,
    excludedSourcesRepo,
  );

  group('FeedManager', () {
    late ActiveDocumentData data;
    late Document doc2, doc3;
    late DocumentId id2, id3;
    final id = DocumentId();

    setUp(() async {
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
      await changedRepo.add(id2);

      engine.resetCallCounter();
    });

    tearDown(() async {
      await docBox.clear();
      await activeBox.clear();
      await changedBox.clear();
      await stateBox.clear();
    });

    test('deactivate documents', () async {
      final evt = await mgr.deactivateDocuments({id2, id3, id});
      expect(evt is ClientEventSucceeded, isTrue);

      // id2 should be removed from active and changed repos
      expect(activeBox, isEmpty);
      expect(changedBox, isEmpty);

      // id2 should now be deactivated, id3 still inactive
      expect(docBox, hasLength(2));
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
          <DocumentId>[engine.doc0.documentId, engine.doc1.documentId],
        ),
      );

      // check repositories are updated
      expect(docBox, hasLength(4));
      expect(docBox.values, contains(engine.doc0));
      expect(docBox.values, contains(engine.doc1));
      expect(activeBox, hasLength(3));
      expect(activeBox.values, contains(engine.active0));
      expect(activeBox.values, contains(engine.active1));

      // serialize should be called and state saved
      expect(engine.getCallCount('serialize'), equals(1));
      expect(stateBox.isNotEmpty, isTrue);
    });

    test('restore feed', () async {
      final earlier = DateTime.utc(1969, 7, 20);
      final later = DateTime.utc(1989, 11, 9);
      await docRepo.update(doc2..timestamp = earlier);
      await docRepo.update(doc3..timestamp = later);
      await docRepo.update(engine.doc0..timestamp = later);
      await docRepo.update(engine.doc1..timestamp = earlier);

      expect(docBox, hasLength(4));

      final evt = await mgr.restoreFeed();
      final feed = evt.whenOrNull(restoreFeedSucceeded: (docs) => docs);

      expect(feed, isNotNull);
      expect(feed, hasLength(3));
      // doc1, doc2 have the earlier timestamp
      expect(feed![0].documentId, equals(engine.doc1.documentId));
      expect(feed[0].batchIndex, equals(1));
      expect(feed[1].documentId, equals(doc2.documentId));
      expect(feed[1].batchIndex, equals(2));
      // doc0 has the later timestamp
      expect(feed[2].documentId, equals(engine.doc0.documentId));
      expect(feed[2].batchIndex, equals(0));
      // doc3 is excluded since it is inactive
    });
  });

  group('Excluded sources', () {
    setUp(() async {
      await excludedSourcesRepo.save({'www.bbc.com'});
    });

    tearDown(() async {
      await excludedBox.clear();
    });

    test('when adding empty source throw "ArgumentError"', () async {
      expect(() => mgr.addExcludedSource(''), throwsArgumentError);
    });

    test('when removing empty source throw "ArgumentError"', () async {
      expect(() => mgr.removeExcludedSource(''), throwsArgumentError);
    });

    test('addExcludedSource', () async {
      final excludedSoures = {'www.bbc.com', 'www.nytimes.com'};
      const source1 = 'www.bbc.com';
      const source2 = 'www.nytimes.com';

      final response1 = await mgr.addExcludedSource(source1);
      final response2 = await mgr.addExcludedSource(source2);

      expect(response1, isA<ClientEventSucceeded>());
      expect(response2, isA<ClientEventSucceeded>());
      expect(excludedBox.values.first, equals(excludedSoures));
    });

    test('removeExcludedSource', () async {
      final excludedSoures = {'www.bbc.com', 'www.nytimes.com'};
      await excludedSourcesRepo.save(excludedSoures);

      final response = await mgr.removeExcludedSource('www.bbc.com');

      expect(response, isA<ClientEventSucceeded>());
      expect(excludedBox.values.first, equals({'www.nytimes.com'}));
    });

    test('getExcludedSourcesList', () async {
      final excludedSoures = {
        'theguardian.com',
        'bbc.co.uk',
        'wsj.com',
        'www.nytimes.com',
      };
      await excludedSourcesRepo.save(excludedSoures);

      final response = await mgr.getExcludedSourcesList();

      expect(response, isA<ExcludedSourcesListRequestSucceeded>());
      expect(
        (response as ExcludedSourcesListRequestSucceeded).excludedSources,
        equals(excludedSoures),
      );
    });
  });
}
