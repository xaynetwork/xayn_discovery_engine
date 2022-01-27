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

import 'dart:typed_data' show Uint8List, BytesBuilder;
import 'package:xayn_discovery_engine/src/domain/assets/asset.dart'
    show Asset, Fragment;
import 'package:xayn_discovery_engine/src/logger.dart' show logger;

/// Fetches the asset either from the `urlSuffix` or from [Fragment]s
/// and returns a single bytes list.
abstract class AssetFetcher {
  Future<Uint8List> fetchFragment(String urlSuffix);
  Future<Uint8List> fetchAsset(Asset asset) async {
    final builder = BytesBuilder(copy: false);

    final message =
        'AssetFetcher:\n  id: ${asset.id},\n  url: ${asset.urlSuffix}\n  hasFragments: ${asset.fragments.isNotEmpty}';
    logger.i(message);

    if (asset.fragments.isEmpty) {
      final bytes = await fetchFragment(asset.urlSuffix);
      builder.add(bytes);
    }

    for (final fragment in asset.fragments) {
      final bytes = await fetchFragment(fragment.urlSuffix);
      builder.add(bytes);
    }

    return builder.takeBytes();
  }
}

/// Thrown when a there is an issue with downloading AI assets.
class AssetFetcherException implements Exception {
  /// Message (or string representation of the exception).
  final String message;

  AssetFetcherException(this.message);

  @override
  String toString() => 'AssetFetcherException: $message';
}
