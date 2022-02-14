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
  test('reading written init config ffi works', () {
    final config = InitConfigFfi.fromParts(
      apiKey: 'hjlsdfhjfdhjk',
      apiBaseUrl: 'https://foo.example/api/v1',
      feedMarkets: [
        const FeedMarket(countryCode: 'DE', langCode: 'DE'),
        const FeedMarket(countryCode: 'US', langCode: 'EN'),
      ],
      smbertVocab: 'foo/bar',
      smbertModel: 'bar/foot',
      kpeVocab: 'do.do',
      kpeModel: 'yo.lo',
      kpeCnn: 'abc',
      kpeClassifier: 'magic',
    );
    final boxed = config.allocNative();
    final res = InitConfigFfi.readNative(boxed.ref);
    boxed.free();
    expect(res, equals(config));
  });
}
