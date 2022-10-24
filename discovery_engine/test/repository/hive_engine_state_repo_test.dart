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

import 'dart:io' show Directory;
import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart' show Hive;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/infrastructure/migration.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_engine_state_repo.dart'
    show HiveEngineStateRepository;

Future<void> main() async {
  group('HiveEngineStateRepository', () {
    late HiveEngineStateRepository repo;

    setUpAll(() async {
      registerHiveAdapters();
    });

    setUp(() async {
      final dir =
          Directory.systemTemp.createTempSync('HiveEngineStateRepository');
      await initDatabase(dir.path);
      repo = HiveEngineStateRepository();
    });

    tearDown(() async {
      await Hive.deleteFromDisk();
    });

    group('"load" method', () {
      test('when the box is empty it will return "null"', () async {
        final state = await repo.load();

        expect(state, isNull);
      });

      test('when the box has some data it will return that data', () async {
        await repo.box.put(
          HiveEngineStateRepository.stateKey,
          Uint8List.fromList([1, 2, 3, 4]),
        );

        final state = await repo.load();

        expect(state, equals(Uint8List.fromList([1, 2, 3, 4])));
      });
    });

    group('"save" method', () {
      test('when we save some data it will be present in the box', () async {
        await repo.save(Uint8List.fromList([5, 6, 7, 8]));

        expect(repo.box.values.first, equals(Uint8List.fromList([5, 6, 7, 8])));
      });

      test(
          'when we call "save" multiple time it will always override the state '
          'so only one entry will exist in the box', () async {
        await repo.save(Uint8List.fromList([1, 2, 3, 4]));
        await repo.save(Uint8List.fromList([5, 6, 7, 8]));
        await repo.save(Uint8List.fromList([0, 1, 0, 1]));

        expect(repo.box.values.first, equals(Uint8List.fromList([0, 1, 0, 1])));
        expect(repo.box.values.length, equals(1));
      });
    });
  });
}
