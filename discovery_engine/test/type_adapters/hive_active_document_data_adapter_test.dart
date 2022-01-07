import 'dart:io' show Directory;
import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart' show Hive, Box;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData, ActiveDocumentDataAdapter;
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart'
    show DocumentViewMode, DocumentViewModeAdapter;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_duration_adapter.dart'
    show DurationAdapter;

void main() {
  group('ActiveDocumentDataAdapter', () {
    late Box<ActiveDocumentData> box;

    setUpAll(() async {
      Hive.init(Directory.current.path);
      Hive.registerAdapter(DurationAdapter());
      Hive.registerAdapter(DocumentViewModeAdapter());
      Hive.registerAdapter(ActiveDocumentDataAdapter());

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
      final value = ActiveDocumentData(Uint8List.fromList([1, 2, 3, 4]))
        ..addViewTime(DocumentViewMode.web, duration);
      final key = await box.add(value);
      final activeData = box.get(key)!;

      expect(box, hasLength(1));
      expect(activeData, equals(value));
    });
  });
}
