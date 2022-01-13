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

import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart' show Hive;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show changedDocumentIdBox;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_changed_document_repo.dart'
    show HiveChangedDocumentRepository;

Future<void> main() async {
  final box =
      await Hive.openBox<Uint8List>(changedDocumentIdBox, bytes: Uint8List(0));
  final repo = HiveChangedDocumentRepository();

  group('ChangedDocumentRepository', () {
    final id1 = DocumentId();
    final id2 = DocumentId();

    tearDown(() async {
      await box.clear();
    });

    group('empty box', () {
      test('add new', () async {
        expect(box, isEmpty);
        await repo.add(id1);
        expect(box, hasLength(1));
      });

      test('fetch all', () async {
        expect(await repo.fetchAll(), isEmpty);
      });

      test('remove all', () async {
        await repo.removeAll();
        expect(box, isEmpty);
      });

      test('remove many', () async {
        await repo.removeMany([id1, id2]);
        expect(box, isEmpty);
      });
    });

    group('nonempty box', () {
      setUp(() async {
        await repo.add(id1);
      });

      test('fetch all from one', () async {
        final all = await repo.fetchAll();
        expect(all, hasLength(1));
        expect(all.first, equals(id1));
      });

      test('add existing and fetch it', () async {
        await repo.add(id1);
        expect(box, hasLength(1));

        final all = await repo.fetchAll();
        expect(all, hasLength(1));
        expect(all.first, equals(id1));
      });

      test('add new and fetch all', () async {
        await repo.add(id2);
        expect(box, hasLength(2));
        final all = await repo.fetchAll();
        expect(all, hasLength(2));
        expect(all, containsAll(<DocumentId>[id1, id2]));
      });

      test('remove all', () async {
        await repo.removeAll();
        expect(box, isEmpty);
      });

      test('remove absent then present', () async {
        await repo.removeMany([id2]);
        expect(box, hasLength(1));

        await repo.removeMany([id1]);
        expect(box, isEmpty);
      });

      test('remove absent then present', () async {
        await repo.removeMany([id1, id2]);
        expect(box, isEmpty);
      });
    });
  });
}
