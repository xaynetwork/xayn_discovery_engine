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
import 'package:xayn_discovery_engine/src/domain/models/source.dart';
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_source_adapter.dart'
    show SetSourceAdapter, SourceAdapter;

import '../logging.dart' show setupLogging;

void main() {
  setupLogging();

  group('SetSourceAdapter', () {
    late Box<Set<Source>> box;

    setUpAll(() async {
      Hive.init(Directory.current.path);
      Hive.registerAdapter(SetSourceAdapter());
      box = await Hive.openBox<Set<Source>>('SetSourceAdapter');
    });

    tearDown(() async {
      await box.clear();
    });

    tearDownAll(() async {
      await box.deleteFromDisk();
    });

    test('can write and read `Set<Source>`', () async {
      final originalSet = {
        Source('foo.test'),
        Source('bar.example'),
        Source('baz.test')
      };
      final key = await box.add(originalSet);
      final newSet = box.get(key)!;

      expect(box, hasLength(1));
      expect(originalSet, equals(newSet));
    });
  });

  group('SourceAdapter', () {
    late Box<Source> box;

    setUpAll(() async {
      Hive.init(Directory.current.path);
      Hive.registerAdapter(SourceAdapter());
      box = await Hive.openBox<Source>('SetSourceAdapter');
    });

    tearDown(() async {
      await box.clear();
    });

    tearDownAll(() async {
      await box.deleteFromDisk();
    });

    test('can write and read `Set<Source>`', () async {
      final originalSet = Source('foo.test');
      final key = await box.add(originalSet);
      final newSet = box.get(key)!;

      expect(box, hasLength(1));
      expect(originalSet, equals(newSet));
    });
  });
}
