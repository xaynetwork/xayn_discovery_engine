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
import 'package:hive/hive.dart' show Hive;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/event_handler.dart'
    show EventHandler;
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    show ActiveSearch, SearchBy;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_search_repo.dart'
    show HiveActiveSearchRepository;

Future<void> main() async {
  group('HiveActiveSearchRepository', () {
    late HiveActiveSearchRepository repo;

    final search = ActiveSearch(
      searchBy: SearchBy.query,
      searchTerm: 'example search query',
      requestedPageNb: 1,
      pageSize: 10,
    );

    setUpAll(() async {
      EventHandler.registerHiveAdapters();
    });

    setUp(() async {
      final dir =
          Directory.systemTemp.createTempSync('HiveActiveSearchRepository');
      await EventHandler.initDatabase(dir.path);
      repo = HiveActiveSearchRepository();
    });

    tearDown(() async {
      await Hive.deleteFromDisk();
    });

    group('"getCurrent" method', () {
      test('when the box is empty it will return "null"', () async {
        final state = await repo.getCurrent();

        expect(state, isNull);
      });

      test('when the box has some data it will return that data', () async {
        await repo.box.put(HiveActiveSearchRepository.stateKey, search);

        final state = await repo.getCurrent();

        expect(state, equals(search));
      });
    });

    group('"clear" method', () {
      test('when the box is empty it should do nothing', () async {
        final clearFuture = repo.clear();
        expect(clearFuture, completion(isNull));
        await clearFuture;
        expect(repo.box.isEmpty, isTrue);
      });

      test('when the box is NOT empty it should clear it', () async {
        await repo.save(search);

        await repo.clear();
        expect(repo.box.isEmpty, isTrue);
      });
    });

    group('"save" method', () {
      test('when the box is empty it should persist data into it', () async {
        await repo.save(search);

        expect(repo.box.isNotEmpty, isTrue);
        expect(repo.box.values.first, equals(search));
        expect(repo.box.values.length, equals(1));
      });

      test('when the box is NOT empty it should override previous data',
          () async {
        await repo.save(search);

        final search2 = ActiveSearch(
          searchBy: SearchBy.query,
          searchTerm: 'foobar',
          requestedPageNb: 123,
          pageSize: 10,
        );
        await repo.save(search2);

        expect(repo.box.isNotEmpty, isTrue);
        expect(repo.box.values.first, equals(search2));
        expect(repo.box.values.length, equals(1));
      });
    });
  });
}
