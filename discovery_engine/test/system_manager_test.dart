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
import 'package:xayn_discovery_engine/src/domain/engine/mock_engine.dart'
    show MockEngine;
import 'package:xayn_discovery_engine/src/domain/event_handler.dart'
    show EventConfig, EventHandler;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/system_manager.dart'
    show SystemManager;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_document_repo.dart'
    show HiveDocumentRepository;

import 'discovery_engine/utils/utils.dart';
import 'logging.dart' show setupLogging;

Future<void> main() async {
  setupLogging();

  group('SystemManager', () {
    late HiveDocumentRepository docRepo;
    late SystemManager mgr;

    final engine = MockEngine();
    final config = EventConfig(maxFeedDocs: 5, maxSearchDocs: 20);

    setUpAll(() async {
      EventHandler.registerHiveAdapters();
    });

    setUp(() async {
      final dir = Directory.systemTemp.createTempSync('SystemManager');
      await EventHandler.initDatabase(dir.path);

      docRepo = HiveDocumentRepository();
      mgr = SystemManager(engine, config, docRepo);

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
      await Hive.deleteFromDisk();
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
