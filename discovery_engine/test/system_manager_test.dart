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
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart';
import 'package:xayn_discovery_engine/src/domain/engine/mock_engine.dart'
    show MockEngine;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/domain/system_manager.dart'
    show SystemManager;

import 'logging.dart' show setupLogging;

Future<void> main() async {
  setupLogging();

  group('SystemManager', () {
    late SystemManager mgr;
    final engine = MockEngine();

    setUp(() {
      mgr = SystemManager(engine, () async {});
    });

    tearDown(() {
      engine.resetCallCounter();
    });

    test('change configuration', () async {
      final markets = {const FeedMarket(langCode: 'de', countryCode: 'DE')};
      final marketResponse = await mgr.changeConfiguration(markets, null, null);
      expect(marketResponse, isA<ClientEventSucceeded>());
      expect(engine.getCallCount('configure'), equals(1));

      final feedResponse = await mgr.changeConfiguration(null, 42, null);
      expect(feedResponse, isA<ClientEventSucceeded>());
      expect(engine.getCallCount('configure'), equals(2));

      final searchResponse = await mgr.changeConfiguration(null, null, 42);
      expect(searchResponse, isA<ClientEventSucceeded>());
      expect(engine.getCallCount('configure'), equals(3));
    });

    test('resetAi resets all AI state holders', () async {
      final response = await mgr.resetAi();
      expect(response, isA<ResetAiSucceeded>());
      expect(engine.getCallCount('resetAi'), equals(1));
    });
  });
}
