// Copyright 2021 Xayn AG
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
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart'
    show DocumentViewMode;
import 'package:xayn_discovery_engine/src/infrastructure/migration.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart'
    show HiveActiveDocumentDataRepository;

import '../logging.dart' show setupLogging;

Future<void> main() async {
  setupLogging();

  group('ActiveDocumentDataRepository', () {
    late HiveActiveDocumentDataRepository repo;
    final data = ActiveDocumentData(Embedding.fromList([]));
    final id1 = DocumentId();
    final id2 = DocumentId();

    setUpAll(() async {
      registerHiveAdapters();
    });

    setUp(() async {
      final dir =
          Directory.systemTemp.createTempSync('ActiveDocumentDataRepository');
      await initDatabase(dir.path);
      repo = HiveActiveDocumentDataRepository();
    });

    tearDown(() async {
      await Hive.deleteFromDisk();
    });

    group('empty box', () {
      test('smbert embedding for absent id', () async {
        expect(await repo.smbertEmbeddingById(id1), isNull);
      });

      test('add new', () async {
        const duration = Duration(seconds: 3);
        data.addViewTime(DocumentViewMode.web, duration);
        await repo.update(id1, data);
        expect(repo.box, hasLength(1));
        expect(repo.box.values.first, equals(data));
      });

      test('fetch none', () async {
        expect(await repo.fetchById(id1), isNull);
      });

      test('remove by ids', () async {
        await repo.removeByIds([id1]);
        expect(repo.box.values, isEmpty);
      });
    });

    group('nonempty box', () {
      setUp(() async {
        await repo.update(id1, data);
      });

      test('existing smbert embedding', () async {
        final emb = await repo.smbertEmbeddingById(id1);
        expect(
          emb,
          equals(
            // ignore: deprecated_member_use_from_same_package
            data.smbertEmbedding,
          ),
        );
      });

      test('smbert embedding of updated existing', () async {
        final embUpdated = Embedding.fromList([9.25]);
        await repo.update(id1, ActiveDocumentData(embUpdated));
        expect(repo.box, hasLength(1));
        expect(await repo.smbertEmbeddingById(id1), equals(embUpdated));
      });

      test('fetch present then absent', () async {
        expect(await repo.fetchById(id1), equals(data));
        expect(await repo.fetchById(id2), isNull);
      });

      test('remove absent then present', () async {
        await repo.removeByIds([id2]);
        expect(repo.box, hasLength(1));

        await repo.removeByIds([id1]);
        expect(repo.box, isEmpty);
      });

      test('remove absent and present', () async {
        await repo.removeByIds([id1, id2]);
        expect(repo.box, isEmpty);
      });
    });
  });
}
