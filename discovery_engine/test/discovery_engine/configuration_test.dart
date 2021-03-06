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
import 'package:xayn_discovery_engine/discovery_engine.dart';

import '../logging.dart' show setupLogging;

void main() {
  setupLogging();

  test(
    'GIVEN empty set of FeedMarket WHEN create a Configuration THEN throw AssertError',
    () {
      const values = <FeedMarket>{};
      expect(
        () => Configuration(
          feedMarkets: values,
          apiKey: '',
          apiBaseUrl: '',
          assetsUrl: '',
          maxItemsPerFeedBatch: -1,
          maxItemsPerSearchBatch: -1,
          applicationDirectoryPath: '',
          manifest: Manifest([]),
          headlinesProviderPath: '/newscatcher/v1/latest-headlines',
          newsProviderPath: '/newscatcher/v1/search-news',
        ),
        throwsA(isA<AssertionError>()),
      );
    },
  );
}
