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

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show DiscoveryEngine, cfgFeatureStorage;

import '../logging.dart' show setupLogging;
import 'utils/helpers.dart'
    show TestEngineData, initEngine, setupTestEngineData;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer, ReplyWith;

void main() {
  setupLogging();

  group('DiscoveryEngine init', () {
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

    test('init engine with ai models', () async {
      final engine = await initEngine(data, server.port);
      expect(engine, isA<DiscoveryEngine>());
      await engine.dispose();
    });

    test('news api request error should not raise an engine init exception',
        () async {
      server.replyWith = ReplyWith.error;
      final engine = await initEngine(data, server.port);
      expect(engine, isA<DiscoveryEngine>());
      await engine.dispose();
    });

    test('db override error is reported when the db was corrupted', () async {
      data.useEphemeralDb = false;
      await File('${data.applicationDirectoryPath}/db.sqlite')
          .writeAsBytes([11, 11, 11, 11, 11, 11, 11, 11], flush: true);
      final engine = await initEngine(data, server.port);

      if (cfgFeatureStorage) {
        expect(engine.lastDbOverrideError, isNotNull);
      } else {
        expect(engine.lastDbOverrideError, isNull);
      }
    });
  });
}
