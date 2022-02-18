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
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData, ActiveDocumentDataAdapter;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart'
    show DocumentViewMode, DocumentViewModeAdapter;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_duration_adapter.dart'
    show DurationAdapter;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_embedding_adapter.dart'
    show EmbeddingAdapter;

import '../logging.dart' show setupLogging;

void main() {
  setupLogging();

  group('ActiveDocumentDataAdapter', () {
    late Box<ActiveDocumentData> box;

    setUpAll(() async {
      Hive.init(Directory.current.path);
      Hive.registerAdapter(DurationAdapter());
      Hive.registerAdapter(DocumentViewModeAdapter());
      Hive.registerAdapter(ActiveDocumentDataAdapter());
      Hive.registerAdapter(EmbeddingAdapter());

      box = await Hive.openBox<ActiveDocumentData>('ActiveDocumentDataAdapter');
    });

    tearDown(() async {
      await box.clear();
    });

    tearDownAll(() async {
      await box.deleteFromDisk();
    });

    test('can write and read `ActiveDocumentData`', () async {
      const duration = Duration(seconds: 3);
      final value = ActiveDocumentData(Embedding.fromList([1, 2, 3, 4]))
        ..addViewTime(DocumentViewMode.web, duration);
      final key = await box.add(value);
      final activeData = box.get(key)!;

      expect(box, hasLength(1));
      expect(activeData, equals(value));
    });
  });
}
