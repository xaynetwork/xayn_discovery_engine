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

import 'dart:io' show Directory, File;

import 'package:test/test.dart';

import 'package:xayn_discovery_engine/discovery_engine.dart'
    show
        AvailableSourcesListRequestFailed,
        AvailableSourcesListRequestSucceeded,
        DiscoveryEngine;
import 'package:xayn_discovery_engine/src/domain/assets/assets.dart'
    show AssetType, kAssetsPath;

import '../logging.dart' show setupLogging;
import 'utils/helpers.dart'
    show TestEngineData, initEngine, setupTestEngineData;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

void main() {
  setupLogging();

  group('DiscoveryEngine getAvailableSourcesList', () {
    late LocalNewsApiServer server;
    late TestEngineData data;
    late DiscoveryEngine engine;

    setUp(() async {
      server = await LocalNewsApiServer.start();
      data = await setupTestEngineData();
      engine = await initEngine(data, server.port);
    });

    tearDown(() async {
      await engine.dispose();
      await server.close();
      await Directory(data.applicationDirectoryPath).delete(recursive: true);
    });

    test('glob search term should return all available sources', () async {
      expect(
        engine.engineEvents,
        emitsInOrder(<Matcher>[isA<AvailableSourcesListRequestSucceeded>()]),
      );
      final response = await engine.getAvailableSourcesList('');
      final lines = await File(
        '${data.applicationDirectoryPath}/$kAssetsPath/${data.manifest.assets.firstWhere(
              (asset) => asset.id == AssetType.availableSources,
            ).urlSuffix}',
      ).readAsLines();
      expect(response, isA<AvailableSourcesListRequestSucceeded>());
      expect(
        (response as AvailableSourcesListRequestSucceeded)
            .availableSources
            .length,
        equals(lines.length),
      );
    });

    test('unavailable search term should return failure', () async {
      expect(
        engine.engineEvents,
        emitsInOrder(<Matcher>[isA<AvailableSourcesListRequestFailed>()]),
      );
      final response = await engine.getAvailableSourcesList('\b\b\b');
      expect(response, isA<AvailableSourcesListRequestFailed>());
    });

    test('example search term should return example related sources', () async {
      expect(
        engine.engineEvents,
        emitsInOrder(<Matcher>[isA<AvailableSourcesListRequestSucceeded>()]),
      );
      const fuzzySearchTerm = 'example';
      final response = await engine.getAvailableSourcesList(fuzzySearchTerm);
      expect(response, isA<AvailableSourcesListRequestSucceeded>());
      final availableSources =
          (response as AvailableSourcesListRequestSucceeded).availableSources;
      expect(availableSources, isNotEmpty);
      for (final availableSource in availableSources) {
        expect(availableSource.name.toLowerCase(), contains(fuzzySearchTerm));
      }
    });
  });
}
