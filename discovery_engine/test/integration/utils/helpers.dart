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

import 'dart:io';

import 'package:xayn_discovery_engine/discovery_engine.dart'
    show
        Configuration,
        DiscoveryEngine,
        FeedMarket,
        Manifest,
        createManifestReader;
import 'package:xayn_discovery_engine/src/domain/assets/assets.dart'
    show kAssetsPath;

class TestEngineData {
  final Manifest manifest;
  final String applicationDirectoryPath;
  TestEngineData(this.manifest, this.applicationDirectoryPath);
}

Future<TestEngineData> setupTestEngineData() async {
  final applicationDirectoryPath =
      (await Directory.systemTemp.createTemp()).path;
  await Link(
    '$applicationDirectoryPath/$kAssetsPath',
  ).create(
    '${Directory.current.path}/../discovery_engine_flutter/example/assets',
    recursive: true,
  );
  final manifest = await createManifestReader().read();

  return TestEngineData(manifest, applicationDirectoryPath);
}

Configuration createConfig(
  TestEngineData data,
  int serverPort,
) {
  return Configuration(
    apiKey: '**********',
    apiBaseUrl: 'http://localhost:$serverPort',
    assetsUrl: 'https://ai-assets.xaynet.dev',
    maxItemsPerFeedBatch: 50,
    maxItemsPerSearchBatch: 20,
    applicationDirectoryPath: data.applicationDirectoryPath,
    feedMarkets: {const FeedMarket(countryCode: 'DE', langCode: 'de')},
    manifest: data.manifest,
  );
}

Future<DiscoveryEngine> initEngine(
  TestEngineData data,
  int serverPort,
) async {
  return DiscoveryEngine.init(
    configuration: createConfig(data, serverPort),
  );
}
