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

import 'package:xayn_discovery_engine/src/domain/assets/data_provider.dart'
    show SetupData;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarkets;

/// Configuration needed to initialize the engine.
class EngineConfig {
  final String apiKey;
  final String apiBaseUrl;
  final FeedMarkets feedMarkets;
  final SetupData setupData;

  EngineConfig({
    required this.apiKey,
    required this.apiBaseUrl,
    required this.feedMarkets,
    required this.setupData,
  });
}
