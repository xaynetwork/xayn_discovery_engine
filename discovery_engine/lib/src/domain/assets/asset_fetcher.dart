import 'dart:typed_data' show Uint8List, BytesBuilder;
import 'package:xayn_discovery_engine/src/domain/assets/asset.dart'
    show Asset, Fragment;

/// Fetches the asset aither from the urlSuffix or from [Fragment]s
/// and returns a single bytes list.
abstract class AssetFetcher {
  Future<Uint8List> fetch(String urlSuffix);
  Future<Uint8List> fetchAsset(Asset asset) async {
    final builder = BytesBuilder(copy: false);

    if (asset.fragments.isEmpty) {
      final bytes = await fetch(asset.urlSuffix);
      builder.add(bytes);
    }

    for (final fragment in asset.fragments) {
      final bytes = await fetch(fragment.urlSuffix);
      builder.add(bytes);
    }

    // Returns the bytes currently contained in this builder and clears it.
    return builder.takeBytes();
  }
}
