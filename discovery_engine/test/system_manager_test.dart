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
import 'package:xayn_discovery_engine/src/domain/event_handler.dart'
    show EventConfig;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, DocumentAdapter;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/system_manager.dart'
    show SystemManager;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show documentBox;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_document_repo.dart'
    show HiveDocumentRepository;

import 'discovery_engine/utils/utils.dart';
import 'logging.dart' show setupLogging;

Future<void> main() async {
  setupLogging();

  Hive.registerAdapter(DocumentAdapter());

  final docBox = await Hive.openBox<Document>(documentBox, bytes: Uint8List(0));

  final engine = MockEngine();
  final config = EventConfig(maxFeedDocs: 5, maxSearchDocs: 20);
  final docRepo = HiveDocumentRepository();

  final mgr = SystemManager(engine, config, docRepo);

  group('SystemManager', () {
    setUp(() async {
      final stackId = StackId();
      final doc2 = Document(
        documentId: DocumentId(),
        stackId: stackId,
        batchIndex: 2,
        resource: mockNewsResource,
        isActive: true,
      );
      final doc3 = Document(
        documentId: DocumentId(),
        stackId: stackId,
        batchIndex: 3,
        resource: mockNewsResource,
        isActive: false,
      );

      await docRepo.updateMany([doc2, doc3]);
      engine.resetCallCounter();
    });

    tearDown(() async {
      await docBox.clear();
    });

    test('change configuration feedMarkets', () async {
      final markets = {
        const FeedMarket(countryCode: 'Country', langCode: 'language')
      };
      final evt = await mgr.changeConfiguration(markets, null, null);
      expect(evt.whenOrNull(clientEventSucceeded: () => null), isNull);
    });

    test('change configuration maxItemsPerFeedBatch', () async {
      final maxDocs = mgr.maxFeedDocs;
      final evt = await mgr.changeConfiguration(null, maxDocs + 1, null);
      expect(evt.whenOrNull(clientEventSucceeded: () => null), isNull);
      expect(mgr.maxFeedDocs, maxDocs + 1);
    });

    test('change configuration maxItemsPerSearchBatch', () async {
      final maxDocs = mgr.maxSearchDocs;
      final evt = await mgr.changeConfiguration(null, null, maxDocs + 1);
      expect(evt.whenOrNull(clientEventSucceeded: () => null), isNull);
      expect(mgr.maxSearchDocs, maxDocs + 1);
    });
  });
}
