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

@Timeout(Duration(seconds: 80))

import 'dart:io' show Directory;

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show TrendingTopicsRequestSucceeded;

import '../logging.dart' show setupLogging;
import 'utils/helpers.dart'
    show TestEngineData, initEngine, setupTestEngineData;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

void main() {
  setupLogging();

  group(
    'DiscoveryEngine requestTrendingTopics',
    () {
      late LocalNewsApiServer server;
      late TestEngineData data;

      setUp(() async {
        data = await setupTestEngineData();
        server = await LocalNewsApiServer.start();
      });

      tearDown(() async {
        await server.close();
        await Directory(data.applicationDirectoryPath).delete(recursive: true);
      });

      test('requestTrendingTopics should return trending topics', () async {
        final engine = await initEngine(data, server.port);

        expect(
          engine.engineEvents,
          emitsInOrder(<Matcher>[
            isA<TrendingTopicsRequestSucceeded>(),
          ]),
        );

        final trendingTopicsResponse = await engine.requestTrendingTopics();
        expect(trendingTopicsResponse, isA<TrendingTopicsRequestSucceeded>());
        expect(
          (trendingTopicsResponse as TrendingTopicsRequestSucceeded).topics,
          isNotEmpty,
        );
      });
    },
    skip: 'TODO: include after TY-2800 is finished',
  );
}
