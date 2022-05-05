// Copyright 2021 Xayn AG
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

import 'package:test/test.dart';

import 'package:xayn_discovery_engine/src/domain/assets/asset.dart'
    show Manifest;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show EngineInitializer;
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/ffi/types/engine.dart'
    show DiscoveryEngineFfi;
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/data_provider.dart'
    show NativeSetupData;

import '../logging.dart' show setupLogging;

void main() {
  setupLogging();

  test('calling async ffi functions works', () async {
    final config = Configuration(
      apiKey: '',
      apiBaseUrl: '',
      assetsUrl: '',
      maxItemsPerFeedBatch: 0,
      maxItemsPerSearchBatch: 0,
      applicationDirectoryPath: '',
      feedMarkets: {const FeedMarket(countryCode: '', langCode: '')},
      manifest: Manifest([]),
    );
    final setupData = NativeSetupData(
      smbertVocab: '',
      smbertModel: '',
      kpeVocab: '',
      kpeModel: '',
      kpeCnn: '',
      kpeClassifier: '',
      availableSources: '',
    );
    expect(
      DiscoveryEngineFfi.initialize(
        EngineInitializer(
          config: config,
          setupData: setupData,
          engineState: null,
          history: [],
          aiConfig: null,
          trustedSources: {},
          excludedSources: {},
        ),
      ),
      allOf(
        throwsException,
        throwsA(
          predicate(
            (exception) =>
                exception.toString().contains('Error while using the ranker'),
          ),
        ),
      ),
    );
  });
}
