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
import 'package:xayn_discovery_engine/src/domain/event_handler.dart';
import 'package:xayn_discovery_engine/src/domain/models/source.dart';
import 'package:xayn_discovery_engine/src/domain/models/source_preference.dart';
import 'package:xayn_discovery_engine/src/domain/repository/source_preference_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_source_preference_repo.dart';

Future<void> main() async {
  group('HiveSourceFiltersRepository', () {
    late SourcePreferenceRepository repo;

    final sources = {
      SourcePreference(Source('example.de'), PreferenceMode.trusted),
      SourcePreference(Source('nytimes.com'), PreferenceMode.excluded),
      SourcePreference(Source('example.com'), PreferenceMode.excluded),
      SourcePreference(Source('include.com'), PreferenceMode.trusted),
    };

    setUpAll(() async {
      EventHandler.registerHiveAdapters();
    });

    setUp(() async {
      final dir = Directory.systemTemp.createTempSync('hive-test');
      await EventHandler.initDatabase(dir.path);
      repo = HiveSourcePreferenceRepository();
    });

    tearDown(() async {
      await Hive.deleteFromDisk();
    });

    test('works when the box is empty', () async {
      expect(await repo.getTrusted(), equals(<String>{}));
      expect(await repo.getTrusted(), equals(<String>{}));
    });

    test('allows to retrieve source preferences', () async {
      for (final el in sources) {
        await repo.save(el);
      }

      final trusted = (await repo.getTrusted()).map((source) => source.value);
      expect(trusted, equals({'example.de', 'include.com'}));

      final excluded = (await repo.getExcluded()).map((source) => source.value);
      expect(excluded, equals({'example.com', 'nytimes.com'}));
    });
  });
}
