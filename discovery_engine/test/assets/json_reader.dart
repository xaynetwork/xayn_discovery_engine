import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/json_reader.dart'
    show JsonManifestReader;

void main() {
  group('read json', () {
    test('read json', () async {
      final reader = await JsonManifestReader().read('../asset_manifest.json');
      expect(1, 1);
    });
  });
}
