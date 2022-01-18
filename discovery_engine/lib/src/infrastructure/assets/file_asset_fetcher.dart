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

import 'dart:io' show File;
import 'dart:typed_data' show Uint8List;

import 'package:xayn_discovery_engine/src/domain/assets/asset_fetcher.dart'
    show AssetFetcher;
import 'package:xayn_discovery_engine/src/domain/assets/data_provider.dart'
    show DataProvider;

class FileAssetFetcher extends AssetFetcher {
  final String baseDirectoryPath;

  FileAssetFetcher(this.baseDirectoryPath);

  @override
  Future<Uint8List> fetchFragment(String urlSuffix) async {
    final path = DataProvider.joinPaths([baseDirectoryPath, urlSuffix]);
    final file = File(path);

    if (!file.existsSync()) {
      final msg = 'Unable to fetch static AI files:\nurl: $path';
      return Future.error(msg);
    }

    return File(path).readAsBytes();
  }
}
