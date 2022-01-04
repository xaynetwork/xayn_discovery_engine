import 'dart:io' show Directory;

import 'package:hive/hive.dart' show Hive, Box;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_duration_adapter.dart'
    show DurationAdapter;

void main() {
  group('DurationAdapter', () {
    late Box<Duration> box;

    setUpAll(() async {
      Hive.registerAdapter(DurationAdapter());
      Hive.init(Directory.current.path);
      box = await Hive.openBox<Duration>('DurationAdapter');
    });

    tearDown(() async {
      await box.clear();
    });

    tearDownAll(() async {
      await box.deleteFromDisk();
    });

    test('can write and read `Duration`', () async {
      const value = Duration(seconds: 42);
      final key = await box.add(value);
      final duration = box.get(key)!;

      expect(box, hasLength(1));
      expect(duration, equals(value));
    });
  });
}
