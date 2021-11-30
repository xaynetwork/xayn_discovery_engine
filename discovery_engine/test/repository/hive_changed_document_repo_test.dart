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

      test('fetch all from none', () async {
        expect(await repo.fetchAll(), isEmpty);
      });

      test('remove all from none', () async {
        await repo.removeAll();
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
    });
  });
}
