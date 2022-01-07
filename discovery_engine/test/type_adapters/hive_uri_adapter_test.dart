import 'dart:io' show Directory;

import 'package:hive/hive.dart' show Hive, Box;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_uri_adapter.dart'
    show UriAdapter;

void main() {
  group('UriAdapter', () {
    late Box<Uri> box;

    setUpAll(() async {
      Hive.init(Directory.current.path);
      Hive.registerAdapter(UriAdapter());
      box = await Hive.openBox<Uri>('UriAdapter');
    });

    tearDown(() async {
      await box.clear();
    });

    tearDownAll(() async {
      await box.deleteFromDisk();
    });

    test('can write and read `Uri`', () async {
      final value =
          Uri.parse('http://example.com:8080/some/url?query=some query');
      final key = await box.add(value);
      final uri = box.get(key)!;

      expect(box, hasLength(1));
      expect(uri, equals(value));
    });
  });
}
