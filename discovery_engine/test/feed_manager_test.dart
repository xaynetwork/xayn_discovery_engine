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
import 'package:xayn_discovery_engine/src/domain/engine/mock_engine.dart'
    show MockEngine;
import 'package:xayn_discovery_engine/src/domain/feed_manager.dart'
    show FeedManager;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, DocumentAdapter, DocumentFeedback;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
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

  final mgr = FeedManager(engine, 5, docRepo, activeRepo, changedRepo);

  group('FeedManager', () {
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

    test('deactivate documents', () async {
      await mgr.deactivateDocuments({id1, id2, id3});

      // id1 should be removed from active and changed repos
      expect(activeBox, isEmpty);
      expect(changedBox, isEmpty);

      // id1 should now be deactivated, id2 still inactive
      expect(docBox, hasLength(2));
      final docs = await docRepo.fetchByIds({id1, id2});
      expect(docs, hasLength(2));
      expect(docs[0].isActive, isFalse);
      expect(docs[1].isActive, isFalse);
    });
  });
}
