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
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show cfgFeatureStorage;
import 'package:xayn_discovery_engine/src/api/api.dart' hide Document;
import 'package:xayn_discovery_engine/src/domain/engine/mock_engine.dart'
    show MockEngine;
import 'package:xayn_discovery_engine/src/domain/event_handler.dart'
    show EventHandler;
import 'package:xayn_discovery_engine/src/domain/feed_manager.dart'
    show FeedManager;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show mockedAvailableSources;
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

  EventHandler.registerHiveAdapters();

  group(
    'FeedManager',
    () {
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
          containsAll(
            engine.feedDocuments.map<Document>((doc) => doc.document),
          ),
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
        // ignore: deprecated_member_use_from_same_package
        await docRepo.update(doc2..timestamp = earlier);
        // ignore: deprecated_member_use_from_same_package
        await docRepo.update(doc3..timestamp = later);
        // ignore: deprecated_member_use_from_same_package
        await docRepo
            // ignore: deprecated_member_use_from_same_package
            .update(engine.feedDocuments[0].document..timestamp = later);
        await docRepo
            // ignore: deprecated_member_use_from_same_package
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
        expect(feed[1].documentId, equals(doc2.documentId));
        // doc0 has the later timestamp
        expect(
          feed[2].documentId,
          equals(engine.feedDocuments[0].document.documentId),
        );
        // doc3 is excluded since it is inactive
      });
    },
    skip: cfgFeatureStorage,
  );
}
