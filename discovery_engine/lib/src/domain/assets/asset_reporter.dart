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

import 'dart:async' show StreamController;

import 'package:xayn_discovery_engine/src/api/api.dart'
    show
        EngineEvent,
        FetchingAssetsFinished,
        FetchingAssetsProgressed,
        FetchingAssetsStarted;
import 'package:xayn_discovery_engine/src/domain/assets/asset.dart'
    show Manifest;

class AssetReporter {
  int _totalNbOfAssets = 0;
  final Set<String> _fetchedUrls = {};
  final _statusCtrl = StreamController<EngineEvent>.broadcast();
  Stream<EngineEvent> get progress => _statusCtrl.stream;

  void fetchingStarted(Manifest manifest) {
    _totalNbOfAssets = manifest.assets.fold<int>(
      0,
      (sum, asset) =>
          sum + (asset.fragments.isNotEmpty ? asset.fragments.length : 1),
    );
    _statusCtrl.add(const FetchingAssetsStarted());
  }

  void assetFetched(String fetchedUrl) {
    assert(_totalNbOfAssets > 0);
    final currentCount = (_fetchedUrls..add(fetchedUrl)).length;
    final progress = currentCount * 100 / _totalNbOfAssets;
    _statusCtrl.add(FetchingAssetsProgressed(progress));
  }

  Future<void> fetchingFinished() async {
    _statusCtrl.add(const FetchingAssetsFinished());
    await _statusCtrl.close();
  }
}
