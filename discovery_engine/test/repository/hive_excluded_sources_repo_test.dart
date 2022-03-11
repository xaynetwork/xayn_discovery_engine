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
    show excludedSourcesBox;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_excluded_sources_repo.dart'
    show HiveExcludedSourcesRepository;

Future<void> main() async {
  group('HiveExcludedSourcesRepository', () {
    late Box<Set<Uri>> box;
    late HiveExcludedSourcesRepository repo;

    final sources = <Uri>{
      Uri(host: 'nytimes.com'),
      Uri(host: 'cnn.com'),
      Uri(host: 'wsj.com'),
    };

    setUpAll(() async {
      box =
          await Hive.openBox<Set<Uri>>(excludedSourcesBox, bytes: Uint8List(0));
    });

    setUp(() async {
      repo = HiveExcludedSourcesRepository();
    });

    tearDown(() async {
      await box.clear();
    });

    group('"getAll" method', () {
      test('when the box is empty it will return an empty set', () async {
        final excludedSources = await repo.getAll();

        expect(excludedSources, equals(<Uri>{}));
      });

      test('when the box has some data it will return that data', () async {
        await box.put(HiveExcludedSourcesRepository.stateKey, sources);

        expect(repo.getAll(), completion(equals(sources)));
      });
    });

    group('"save" method', () {
      test('when the box is empty it should persist data into it', () async {
        await repo.save(sources);

        expect(box.isNotEmpty, isTrue);
        expect(box.values.first, equals(sources));
        expect(box.values.length, equals(1));
      });

      test('when the box is NOT empty it should override previous data',
          () async {
        await repo.save(sources);

        final newSources = <Uri>{
          Uri(host: 'theguardian.com'),
          Uri(host: 'bbc.co.uk'),
        };
        await repo.save(newSources);

        expect(box.isNotEmpty, isTrue);
        expect(box.values.first, equals(newSources));
        expect(box.values.length, equals(1));
      });
    });
  });
}
