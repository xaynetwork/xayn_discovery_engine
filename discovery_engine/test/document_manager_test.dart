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

import 'package:hive/hive.dart' show Hive;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/document_manager.dart'
    show DocumentManager;
import 'package:xayn_discovery_engine/src/domain/engine/mock_engine.dart'
    show MockEngine;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show DocumentAdapter, Document, DocumentFeedback;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart'
    show DocumentViewMode;
import 'package:xayn_discovery_engine/src/domain/models/web_resource.dart'
    show WebResource;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show documentBox, activeDocumentDataBox, changedDocumentIdBox;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart'
    show HiveActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_changed_document_repo.dart'
    show HiveChangedDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_document_repo.dart'
    show HiveDocumentRepository;

Future<void> main() async {
  Hive.registerAdapter(DocumentAdapter());

  final docBox = await Hive.openBox<Document>(documentBox, bytes: Uint8List(0));
  final activeBox = await Hive.openBox<ActiveDocumentData>(
    activeDocumentDataBox,
    bytes: Uint8List(0),
  );
  final changedBox =
      await Hive.openBox<Uint8List>(changedDocumentIdBox, bytes: Uint8List(0));

  final engine = MockEngine();
  final docRepo = HiveDocumentRepository();
  final activeRepo = HiveActiveDocumentDataRepository();
  final changedRepo = HiveChangedDocumentRepository();

  final mgr = DocumentManager(engine, docRepo, activeRepo);

  group('DocumentManager', () {
    final data = ActiveDocumentData(Uint8List(0));
    final dummy = WebResource.fromJson(<String, Object>{
      'title': 'Example',
      'displayUrl': 'domain.com',
      'snippet': 'snippet',
      'url': 'http://domain.com',
      'datePublished': '1980-01-01T00:00:00.000000',
      'provider': <String, String>{
        'name': 'domain',
        'thumbnail': 'http://thumbnail.domain.com',
      },
    });
    final stackId = StackId();
    final doc1 = Document(
      stackId: stackId,
      personalizedRank: 0,
      webResource: dummy,
      isActive: true,
    );
    final doc2 = Document(
      stackId: stackId,
      personalizedRank: 1,
      webResource: dummy,
      isActive: false,
    );
    final id1 = doc1.documentId;
    final id2 = doc2.documentId;
    final id3 = DocumentId();

    setUp(() async {
      // doc1 is active & changed, doc2 is neither
      await docRepo.updateMany([doc1, doc2]);
      await activeRepo.update(id1, data);
      await changedRepo.add(id1);

      engine.resetCallCounter();
    });

    tearDown(() async {
      await docBox.clear();
      await activeBox.clear();
      await changedBox.clear();

      // reset test data
      doc1.isActive = true;
      doc1.feedback = DocumentFeedback.neutral;
      doc2.isActive = false;
      doc2.feedback = DocumentFeedback.neutral;
      data.viewTime.clear();
    });

    test('update absent document feedback', () async {
      expect(
        () => mgr.updateDocumentFeedback(id3, DocumentFeedback.positive),
        throwsArgumentError,
      );
    });

    test('update inactive document feedback', () async {
      expect(
        () => mgr.updateDocumentFeedback(id2, DocumentFeedback.positive),
        throwsArgumentError,
      );
    });

    test(
        'if there is no smbert embedding associated with the document '
        'it should throw StateError', () async {
      // let's get rid of active document data of doc1
      await activeRepo.removeByIds({id1});

      expect(
        () => mgr.updateDocumentFeedback(id1, DocumentFeedback.positive),
        throwsStateError,
      );
    });

    test('update active document feedback', () async {
      const newFeedback = DocumentFeedback.positive;
      await mgr.updateDocumentFeedback(id1, newFeedback);
      expect(engine.getCallCount('userReacted'), equals(1));
      expect(
        docBox.values,
        unorderedEquals(<Document>[doc1..feedback = newFeedback, doc2]),
      );
      // other repos unchanged
      expect(activeBox, hasLength(1));
      expect(await activeRepo.fetchById(id1), equals(data));
      expect(await changedRepo.fetchAll(), equals([id1]));
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

    test('add positive time to document with active data', () async {
      const mode = DocumentViewMode.story;

      // active repo contains just {id1: data} where data satisfies:
      expect(data.getViewTime(mode), Duration.zero);

      // add 5 seconds to id1
      await mgr.addActiveDocumentTime(id1, mode, 5);

      expect(activeBox, hasLength(1));
      var dataUpdated = await activeRepo.fetchById(id1);
      expect(dataUpdated, isNotNull);
      expect(dataUpdated!.smbertEmbedding, equals(data.smbertEmbedding));
      expect(dataUpdated.getViewTime(mode), equals(const Duration(seconds: 5)));

      // other repos unchanged
      expect(await docRepo.fetchAll(), unorderedEquals(<Document>[doc1, doc2]));
      expect(await changedRepo.fetchAll(), equals([id1]));

      // add a further 3 seconds
      await mgr.addActiveDocumentTime(id1, mode, 3);

      expect(activeBox, hasLength(1));
      dataUpdated = await activeRepo.fetchById(id1);
      expect(dataUpdated, isNotNull);
      expect(dataUpdated!.smbertEmbedding, equals(data.smbertEmbedding));
      expect(dataUpdated.getViewTime(mode), equals(const Duration(seconds: 8)));
      expect(engine.getCallCount('timeLogged'), equals(2));
    });
  });
}
