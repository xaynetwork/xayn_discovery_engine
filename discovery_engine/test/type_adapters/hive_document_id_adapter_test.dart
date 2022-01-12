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
      Hive.init(Directory.current.path);
      Hive.registerAdapter(DocumentIdAdapter());
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
