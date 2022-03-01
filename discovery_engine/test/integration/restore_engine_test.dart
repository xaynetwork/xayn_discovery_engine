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

@Timeout(Duration(seconds: 60))

import 'dart:io';
import 'dart:typed_data' show Uint8List;

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show DiscoveryEngine, EngineInitException, NextFeedBatchRequestSucceeded;
import '../logging.dart' show setupLogging;
import 'utils/create_config.dart'
    show TestEngineData, createConfig, setupTestEngineData;
import 'utils/db.dart' show saveEngineState;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

void main() {
  setupLogging();

  group('DiscoveryEngine restore state', () {
    late LocalNewsApiServer server;
    late TestEngineData data;

    setUp(() async {
      data = await setupTestEngineData();
    });

    tearDown(() async {
      await server.close();
      await Directory(data.applicationDirectoryPath).delete(recursive: true);
    });

    test('init engine from a valid state', () async {
      server = await LocalNewsApiServer.start();
      final engine = await DiscoveryEngine.init(
        configuration: createConfig(data, server.port),
      );

      final nextFeedBatchResponse = await engine.requestNextFeedBatch();
      expect(nextFeedBatchResponse, isA<NextFeedBatchRequestSucceeded>());

      await engine.dispose();

      final restoredEngine = await DiscoveryEngine.init(
        configuration: createConfig(data, server.port),
      );
      expect(restoredEngine, isA<DiscoveryEngine>());
    });

    test(
        'init the engine from an invalid state should raise an engine init'
        ' exception', () async {
      server = await LocalNewsApiServer.start();
      await saveEngineState(data.applicationDirectoryPath, Uint8List(0));

      expect(
        () async => DiscoveryEngine.init(
          configuration: createConfig(data, server.port),
        ),
        throwsA(isA<EngineInitException>()),
      );
    });
  });
}
