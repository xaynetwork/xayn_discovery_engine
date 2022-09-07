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

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/ffi/types/init_config.dart'
    show InitConfigFfi;

void main() {
  test('reading written init config ffi without ai config works', () {
    final config = InitConfigFfi.fromParts(
      apiKey: 'hjlsdfhjfdhjk',
      apiBaseUrl: 'https://foo.example/api/v1',
      headlinesProviderPath: '/newscatcher/v1/latest-headlines',
      newsProviderPath: '/newscatcher/v1/search-news',
      feedMarkets: [
        const FeedMarket(langCode: 'de', countryCode: 'DE'),
        const FeedMarket(langCode: 'en', countryCode: 'US'),
      ],
      trustedSources: [],
      excludedSources: [],
      smbertVocab: 'foo/bar',
      smbertModel: 'bar/foot',
      kpeVocab: 'do.do',
      kpeModel: 'yo.lo',
      kpeCnn: 'abc',
      kpeClassifier: 'magic',
      maxDocsPerFeedBatch: 2,
      maxDocsPerSearchBatch: 20,
      dataDir: 'foo/bar',
      useEphemeralDb: false,
    );
    final boxed = config.allocNative();
    final res = InitConfigFfi.readNative(boxed.ref);
    boxed.free();
    expect(res, equals(config));
  });

  test('reading written init config ffi with ai config works', () {
    final config = InitConfigFfi.fromParts(
      apiKey: 'hjlsdfhjfdhjk',
      apiBaseUrl: 'https://foo.example/api/v1',
      headlinesProviderPath: '/newscatcher/v1/latest-headlines',
      newsProviderPath: '/newscatcher/v1/search-news',
      feedMarkets: [
        const FeedMarket(langCode: 'de', countryCode: 'DE'),
        const FeedMarket(langCode: 'en', countryCode: 'US'),
      ],
      trustedSources: [],
      excludedSources: [],
      smbertVocab: 'foo/bar',
      smbertModel: 'bar/foot',
      kpeVocab: 'do.do',
      kpeModel: 'yo.lo',
      kpeCnn: 'abc',
      kpeClassifier: 'magic',
      maxDocsPerFeedBatch: 2,
      maxDocsPerSearchBatch: 20,
      deConfig: '{ "key": "value" }',
      dataDir: 'bar/foo',
      useEphemeralDb: true,
    );
    final boxed = config.allocNative();
    final res = InitConfigFfi.readNative(boxed.ref);
    boxed.free();
    expect(res, equals(config));
  });
}
