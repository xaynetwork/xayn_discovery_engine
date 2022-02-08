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

import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart' show Box, Hive;
import 'package:test/test.dart';

import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show engineStateBox;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_engine_state_repo.dart'
    show HiveEngineStateRepository;

Future<void> main() async {
  group('HiveEngineStateRepository', () {
    late Box<Uint8List> _box;
    final _repo = HiveEngineStateRepository();

    setUpAll(() async {
      _box = await Hive.openBox<Uint8List>(engineStateBox, bytes: Uint8List(0));
    });

    tearDown(() async {
      await _box.clear();
    });

    group('"load" method', () {
      test('when the box is empty it will return "null"', () async {
        final state = await _repo.load();

        expect(state, isNull);
      });

      test('when the box has some data it will return that data', () async {
        await _box.put(
          HiveEngineStateRepository.stateKey,
          Uint8List.fromList([1, 2, 3, 4]),
        );

        final state = await _repo.load();

        expect(state, equals(Uint8List.fromList([1, 2, 3, 4])));
      });
    });

    group('"save" method', () {
      test('when we save some data it will be present in the box', () async {
        await _repo.save(Uint8List.fromList([5, 6, 7, 8]));

        expect(_box.values.first, equals(Uint8List.fromList([5, 6, 7, 8])));
      });

      test(
          'when we call "save" multiple time it will always override the state '
          'so only one entry will exist in the box', () async {
        await _repo.save(Uint8List.fromList([1, 2, 3, 4]));
        await _repo.save(Uint8List.fromList([5, 6, 7, 8]));
        await _repo.save(Uint8List.fromList([0, 1, 0, 1]));

        expect(_box.values.first, equals(Uint8List.fromList([0, 1, 0, 1])));
        expect(_box.values.length, equals(1));
      });
    });
  });
}
