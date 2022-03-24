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

import 'package:hive/hive.dart' show Hive, Box;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_set_string_adapter.dart'
    show SetStringAdapter;

import '../logging.dart' show setupLogging;

void main() {
  setupLogging();

  group('StringSetAdapter', () {
    late Box<Set<String>> box;

    setUpAll(() async {
      Hive.init(Directory.current.path);
      Hive.registerAdapter(SetStringAdapter());
      box = await Hive.openBox<Set<String>>('SetStringAdapter');
    });

    tearDown(() async {
      await box.clear();
    });

    tearDownAll(() async {
      await box.deleteFromDisk();
    });

    test('can write and read `Set<String>`', () async {
      final originalSet = {'foo', 'bar', 'baz'};
      final key = await box.add(originalSet);
      final newSet = box.get(key)!;

      expect(box, hasLength(1));
      expect(originalSet, equals(newSet));
    });
  });
}
