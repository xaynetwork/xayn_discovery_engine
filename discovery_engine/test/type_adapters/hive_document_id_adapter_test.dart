import 'dart:io' show Directory;

import 'package:hive/hive.dart' show Hive, Box;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_document_id_adapter.dart'
    show DocumentIdAdapter;

void main() {
  group('DocumentIdAdapter', () {
    late Box<DocumentId> box;

    setUpAll(() async {
      Hive.registerAdapter(DocumentIdAdapter());
      Hive.init(Directory.current.path);
      box = await Hive.openBox<DocumentId>('DocumentIdAdapter');
    });

    tearDown(() async {
      await box.clear();
    });

    tearDownAll(() async {
      await box.deleteFromDisk();
    });

    test('can write and read `DocumentId`', () async {
      final value = DocumentId();
      final key = await box.add(value);
      final documentId = box.get(key)!;

      expect(box, hasLength(1));
      expect(documentId, equals(value));
    });
  });
}
