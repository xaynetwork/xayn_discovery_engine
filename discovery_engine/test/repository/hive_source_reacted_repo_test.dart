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

import 'package:hive/hive.dart';
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/source.dart';
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart';
import 'package:xayn_discovery_engine/src/infrastructure/migration.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_source_reacted_repo.dart';

Future<void> main() async {
  group('HiveSourceReactedRepository', () {
    late HiveSourceReactedRepository repo;

    final sources = [
      SourceReacted(Source('sub.example.net'), true),
      SourceReacted(Source('example.org'), false),
      SourceReacted(Source('example.com'), true),
      SourceReacted(Source('example.net'), false),
    ];

    setUpAll(() async {
      registerHiveAdapters();
    });

    setUp(() async {
      final dir = Directory.systemTemp.createTempSync('hive-test');
      await initDatabase(dir.path);
      repo = HiveSourceReactedRepository();
    });

    tearDown(() async {
      await Hive.deleteFromDisk();
    });

    test('operations on empty repository', () async {
      expect(await repo.fetchByReaction(true), isEmpty);
      expect(await repo.fetchByReaction(false), isEmpty);

      final source = Source('example.com');
      await repo.remove(source);
      expect(repo.box, isEmpty);

      await repo.save(SourceReacted(source, true));
      expect(repo.box, hasLength(1));
    });

    test('remove one source out of two', () async {
      await repo.save(sources[0]);
      await repo.save(sources[1]);

      await repo.remove(Source('sub.example.net'));
      expect(repo.box, hasLength(1));
      expect(repo.box.values.first.source.value, equals('example.org'));
    });

    test('fetch sources by reaction', () async {
      for (final source in sources) {
        await repo.save(source);
      }

      final likes = await repo.fetchByReaction(true);
      expect(
        likes.map((source) => source.source.value),
        containsAll(<String>['sub.example.net', 'example.com']),
      );

      final dislikes = await repo.fetchByReaction(false);
      expect(
        dislikes.map((source) => source.source.value).toSet(),
        containsAll(<String>['example.org', 'example.net']),
      );
    });
  });
}
