import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart';
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData, ActiveDocumentDataAdapter;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show activeDocumentDataBox;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart'
    show HiveActiveDocumentDataRepository;

void main() async {
  Hive.registerAdapter(ActiveDocumentDataAdapter());
  final box = await Hive.openBox<ActiveDocumentData>(activeDocumentDataBox,
      bytes: Uint8List(0));
  final repo = HiveActiveDocumentDataRepository();

  group('ActiveDocumentDataRepository', () {
    final data = ActiveDocumentData(smbertEmbedding: Uint8List(0));
    final id1 = DocumentId();
    final id2 = DocumentId();

    test('distinct ids', () {
      expect(id1, isNot(id2));
    });

    group('empty box', () {
      tearDown(() async {
        await box.clear();
      });

      test('smbert embedding from none', () async {
        expect(await repo.smbertEmbeddingById(id1), isNull);
      });

      test('add new', () async {
        await repo.update(id1, data);
        expect(box, hasLength(1));
        expect(box.values.first, equals(data));
      });
    });

    group('nonempty box', () {
      setUp(() async {
        await repo.update(id1, data);
      });

      tearDown(() async {
        await box.clear();
      });

      test('existing smbert embedding', () async {
        final emb = await repo.smbertEmbeddingById(id1);
        expect(emb, equals(data.smbertEmbedding));
      });

      test('smbert embedding for absent id', () async {
        expect(await repo.smbertEmbeddingById(id2), isNull);
      });

      test('smbert embedding of updated existing', () async {
        final embUpdated = Uint8List(1);
        await repo.update(id1, ActiveDocumentData(smbertEmbedding: embUpdated));
        expect(box, hasLength(1));
        expect(await repo.smbertEmbeddingById(id1), embUpdated);
      });
    });
  });
}
