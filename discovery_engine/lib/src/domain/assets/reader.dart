import 'package:xayn_discovery_engine/src/domain/assets/asset.dart'
    show Manifest;

abstract class ManifestReader {
  Future<Manifest> read(String path);
}
