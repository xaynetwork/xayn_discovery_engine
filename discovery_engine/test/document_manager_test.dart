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
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart'
    show DocumentsUpdated;
import 'package:xayn_discovery_engine/src/domain/changed_documents_reporter.dart'
    show ChangedDocumentsReporter;
import 'package:xayn_discovery_engine/src/domain/document_manager.dart'
    show DocumentManager;
import 'package:xayn_discovery_engine/src/domain/engine/mock_engine.dart'
    show MockEngine;
import 'package:xayn_discovery_engine/src/domain/event_handler.dart'
    show EventHandler;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart'
    show DocumentViewMode;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart'
    show HiveActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_document_repo.dart'
    show HiveDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_engine_state_repo.dart'
    show HiveEngineStateRepository;

import 'discovery_engine/utils/utils.dart';
import 'logging.dart' show setupLogging;

Future<void> main() async {
  setupLogging();

  late MockEngine engine;
  late HiveDocumentRepository docRepo;
  late HiveActiveDocumentDataRepository activeRepo;
  late HiveEngineStateRepository engineStateRepo;

  group('DocumentManager', () {
    final data = ActiveDocumentData(Embedding.fromList([4, 1]));
    final stackId = StackId();
    final doc1 = Document(
      documentId: DocumentId(),
      stackId: stackId,
      batchIndex: 0,
      resource: mockNewsResource,
      isActive: true,
    );
    final doc2 = Document(
      documentId: DocumentId(),
      stackId: stackId,
      batchIndex: 1,
      resource: mockNewsResource,
      isActive: false,
    );
    final id1 = doc1.documentId;
    final id2 = doc2.documentId;
    final id3 = DocumentId();

    late ChangedDocumentsReporter changedDocsReporter;
    late DocumentManager mgr;

    setUpAll(() async {
      EventHandler.registerHiveAdapters();
    });

    setUp(() async {
      final dir = Directory.systemTemp.createTempSync('DocumentManager');
      await EventHandler.initDatabase(dir.path);

      engine = MockEngine();
      docRepo = HiveDocumentRepository();
      activeRepo = HiveActiveDocumentDataRepository();
      engineStateRepo = HiveEngineStateRepository();
      changedDocsReporter = ChangedDocumentsReporter();
      mgr = DocumentManager(
        engine,
        docRepo,
        activeRepo,
        engineStateRepo,
        changedDocsReporter,
      );

      // doc1 is active & changed, doc2 is neither
      await docRepo.updateMany([doc1, doc2]);
      await activeRepo.update(id1, data);

      engine.resetCallCounter();
    });

    tearDown(() async {
      await changedDocsReporter.close();

      await Hive.deleteFromDisk();

      // reset test data
      doc1.isActive = true;
      doc1.userReaction = UserReaction.neutral;
      doc2.isActive = false;
      doc2.userReaction = UserReaction.neutral;
      data.viewTime.clear();
    });

    test('update absent document user reaction', () async {
      expect(
        () => mgr.updateUserReaction(id3, UserReaction.positive),
        throwsArgumentError,
      );
      expect(changedDocsReporter.changedDocuments, neverEmits(anything));
      await changedDocsReporter.close();
    });

    test('update inactive document user reaction', () async {
      expect(
        () => mgr.updateUserReaction(id2, UserReaction.positive),
        throwsArgumentError,
      );
      expect(changedDocsReporter.changedDocuments, neverEmits(anything));
      await changedDocsReporter.close();
    });

    test(
        'if there is no smbert embedding associated with the document '
        'it should throw StateError', () async {
      // let's get rid of active document data of doc1
      await activeRepo.removeByIds({id1});

      expect(
        () => mgr.updateUserReaction(id1, UserReaction.positive),
        throwsStateError,
      );
      expect(changedDocsReporter.changedDocuments, neverEmits(anything));
      await changedDocsReporter.close();
    });

    test('update active document user reaction', () async {
      const newReaction = UserReaction.positive;
      final updatedDoc = (doc1..userReaction = newReaction).toApiDocument();

      expect(
        changedDocsReporter.changedDocuments,
        emits(equals(DocumentsUpdated([updatedDoc]))),
      );

      await mgr.updateUserReaction(id1, newReaction);

      expect(engine.getCallCount('userReacted'), equals(1));
      expect(
        docRepo.box.values,
        unorderedEquals(<Document>[doc1..userReaction = newReaction, doc2]),
      );
      // serialize should be called and state saved
      expect(engine.getCallCount('serialize'), equals(1));
      expect(engineStateRepo.box.isNotEmpty, isTrue);
      // other repos unchanged
      expect(activeRepo.box, hasLength(1));
      expect(await activeRepo.fetchById(id1), equals(data));
    });

    test('add negative document time', () async {
      const mode = DocumentViewMode.story;
      expect(() => mgr.addActiveDocumentTime(id1, mode, -1), throwsRangeError);
    });

    test('add time to document without active data', () async {
      const mode = DocumentViewMode.story;
      expect(
        () => mgr.addActiveDocumentTime(id2, mode, 1),
        throwsArgumentError,
      );
    });

    test('add time to an inactive document with active document data',
        () async {
      const mode = DocumentViewMode.story;
      await activeRepo.update(id2, data);
      expect(
        () => mgr.addActiveDocumentTime(id2, mode, 1),
        throwsArgumentError,
      );
    });

    test('add time to an absent document with active document data', () async {
      const mode = DocumentViewMode.story;
      await activeRepo.update(id3, data);
      expect(
        () => mgr.addActiveDocumentTime(id3, mode, 1),
        throwsArgumentError,
      );
    });

    test('add positive time to document with active data', () async {
      const mode = DocumentViewMode.story;

      // active repo contains just {id1: data} where data satisfies:
      expect(data.getViewTime(mode), Duration.zero);

      // add 5 seconds to id1
      await mgr.addActiveDocumentTime(id1, mode, 5);

      expect(activeRepo.box, hasLength(1));

      // serialize should be called and state saved
      expect(engine.getCallCount('serialize'), equals(1));
      expect(engineStateRepo.box.isNotEmpty, isTrue);

      var dataUpdated = await activeRepo.fetchById(id1);
      expect(dataUpdated, isNotNull);
      expect(dataUpdated!.smbertEmbedding, equals(data.smbertEmbedding));
      expect(dataUpdated.getViewTime(mode), equals(const Duration(seconds: 5)));

      // other repos unchanged
      expect(await docRepo.fetchAll(), unorderedEquals(<Document>[doc1, doc2]));

      // add a further 3 seconds
      await mgr.addActiveDocumentTime(id1, mode, 3);

      expect(activeRepo.box, hasLength(1));
      expect(engine.getCallCount('serialize'), equals(2));
      dataUpdated = await activeRepo.fetchById(id1);
      expect(dataUpdated, isNotNull);
      expect(dataUpdated!.smbertEmbedding, equals(data.smbertEmbedding));
      expect(dataUpdated.getViewTime(mode), equals(const Duration(seconds: 8)));
      expect(engine.getCallCount('timeSpent'), equals(2));
    });
  });
}
