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

/// Fetches the asset aither from the urlSuffix or from [Fragment]s
/// and returns a single bytes list.
abstract class AssetFetcher {
  Future<Uint8List> fetchFragment(String urlSuffix);
  Future<Uint8List> fetchAsset(Asset asset) async {
    final builder = BytesBuilder(copy: false);

    if (asset.fragments.isEmpty) {
      final bytes = await fetchFragment(asset.urlSuffix);
      builder.add(bytes);
    }

    for (final fragment in asset.fragments) {
      final bytes = await fetchFragment(fragment.urlSuffix);
      builder.add(bytes);
    }

    // Returns the bytes currently contained in this builder and clears it.
    return builder.takeBytes();
  }
}
