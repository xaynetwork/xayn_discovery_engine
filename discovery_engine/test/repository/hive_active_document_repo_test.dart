import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart' show Hive;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart'
    show DocumentViewMode;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show activeDocumentDataBox;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart'
    show HiveActiveDocumentDataRepository;

Future<void> main() async {
  final box = await Hive.openBox<ActiveDocumentData>(
    activeDocumentDataBox,
    bytes: Uint8List(0),
  );
  final repo = HiveActiveDocumentDataRepository();

  group('ActiveDocumentDataRepository', () {
    final data = ActiveDocumentData(Uint8List(0));
    final id1 = DocumentId();
    final id2 = DocumentId();

    tearDown(() async {
      await box.clear();
    });

    group('empty box', () {
      test('smbert embedding for absent id', () async {
        expect(await repo.smbertEmbeddingById(id1), isNull);
      });

      test('add new', () async {
        const duration = Duration(seconds: 3);
        data.addViewTime(DocumentViewMode.web, duration);
        await repo.update(id1, data);
        expect(box, hasLength(1));
        expect(box.values.first, equals(data));
      });

      test('fetch none', () async {
        expect(await repo.fetchById(id1), isNull);
      });

      test('remove by ids', () async {
        await repo.removeByIds([id1]);
        expect(box.values, isEmpty);
      });
    });

    group('nonempty box', () {
      setUp(() async {
        await repo.update(id1, data);
      });

      test('existing smbert embedding', () async {
        final emb = await repo.smbertEmbeddingById(id1);
        expect(emb, equals(data.smbertEmbedding));
      });

      test('smbert embedding of updated existing', () async {
        final embUpdated = Uint8List(1);
        await repo.update(id1, ActiveDocumentData(embUpdated));
        expect(box, hasLength(1));
        expect(await repo.smbertEmbeddingById(id1), equals(embUpdated));
      });

      test('fetch present then absent', () async {
        expect(await repo.fetchById(id1), equals(data));
        expect(await repo.fetchById(id2), isNull);
      });

      test('remove absent then present', () async {
        await repo.removeByIds([id2]);
        expect(box, hasLength(1));

        await repo.removeByIds([id1]);
        expect(box, isEmpty);
      });

      test('remove absent and present', () async {
        await repo.removeByIds([id1, id2]);
        expect(box, isEmpty);
      });
    });
  });
}
