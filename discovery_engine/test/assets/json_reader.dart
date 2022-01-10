import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/json_reader.dart'
    show JsonManifestReader;

void main() {
  group('JsonManifestReader', () {
    test('read json', () async {
      final manifest =
          await JsonManifestReader().read('../asset_manifest.json');
      expect(manifest.assets, isNotEmpty);
    });
  });
}
