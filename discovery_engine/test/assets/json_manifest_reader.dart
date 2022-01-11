import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/json_manifest_reader.dart'
    show JsonManifestReader;

void main() {
  group('JsonManifestReader', () {
    group('read', () {
      test('when given a  ', () async {
        final manifest =
            await JsonManifestReader().read('../asset_manifest.json');

        expect(manifest.assets, isNotEmpty);
      });
    });
  });
}
