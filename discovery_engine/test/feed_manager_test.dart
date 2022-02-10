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
    show ClientEventSucceeded;
import 'package:xayn_discovery_engine/src/domain/engine/mock_engine.dart'
    show MockEngine;
import 'package:xayn_discovery_engine/src/domain/feed_manager.dart'
    show FeedManager;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, DocumentAdapter;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show
        documentBox,
        activeDocumentDataBox,
        changedDocumentIdBox,
        engineStateBox;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart'
    show HiveActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_changed_document_repo.dart'
    show HiveChangedDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_document_repo.dart'
    show HiveDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_engine_state_repo.dart'
    show HiveEngineStateRepository;

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
  await Hive.openBox<Uint8List>(engineStateBox, bytes: Uint8List(0));

  final engine = MockEngine();
  const maxBatch = 5;
  final docRepo = HiveDocumentRepository();
  final activeRepo = HiveActiveDocumentDataRepository();
  final changedRepo = HiveChangedDocumentRepository();
  final engineStateRepo = HiveEngineStateRepository();

  final mgr = FeedManager(
    engine,
    maxBatch,
    docRepo,
    activeRepo,
    changedRepo,
    engineStateRepo,
  );

  group('FeedManager', () {
    late ActiveDocumentData data;
    late Document doc2, doc3;
    late DocumentId id2, id3;
    final id = DocumentId();

    setUp(() async {
      data = ActiveDocumentData(Uint8List(0));
      final stackId = StackId();
      doc2 = Document(
        stackId: stackId,
        batchIndex: 2,
        resource: mockNewsResource,
        isActive: true,
      );
      doc3 = Document(
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
      final feed = evt.whenOrNull(feedRequestSucceeded: (docs) => docs);

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
}
