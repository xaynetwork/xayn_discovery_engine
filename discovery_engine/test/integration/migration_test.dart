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
import 'dart:typed_data';

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show
        DiscoveryEngine,
        DocumentId,
        DocumentViewMode,
        EngineEvent,
        NewsResource,
        RestoreActiveSearchSucceeded,
        RestoreFeedSucceeded,
        Source,
        StackId,
        cfgFeatureStorage;
import 'package:xayn_discovery_engine/src/api/models/document.dart'
    show DocumentApiConversion;
import 'package:xayn_discovery_engine/src/domain/event_handler.dart';
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart';
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart';
import 'package:xayn_discovery_engine/src/domain/models/source_preference.dart';
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_search_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_document_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_engine_state_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_source_preference_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_source_reacted_repo.dart';

import '../logging.dart' show setupLogging;
import 'utils/helpers.dart'
    show TestEngineData, expectEvent, initEngine, setupTestEngineData;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

class HiveInjection {
  final HiveEngineStateRepository engineStateRepository;
  final HiveDocumentRepository documentRepository;
  final HiveActiveSearchRepository activeSearchRepository;
  final HiveActiveDocumentDataRepository activeDocumentDataRepository;
  final HiveSourceReactedRepository sourceReactedRepository;
  final HiveSourcePreferenceRepository sourcePreferenceRepository;

  HiveInjection._(
    this.engineStateRepository,
    this.documentRepository,
    this.activeSearchRepository,
    this.activeDocumentDataRepository,
    this.sourceReactedRepository,
    this.sourcePreferenceRepository,
  );

  static Future<void> accessRepositories(
    String dataDir,
    Future<void> Function(HiveInjection) func,
  ) async {
    final self = await HiveInjection.open(dataDir);
    await func(self);
    await self.close();
  }

  static Future<HiveInjection> open(String dataDir) async {
    EventHandler.registerHiveAdapters();
    await EventHandler.initDatabase(dataDir);
    return HiveInjection._(
      HiveEngineStateRepository(),
      HiveDocumentRepository(),
      HiveActiveSearchRepository(),
      HiveActiveDocumentDataRepository(),
      HiveSourceReactedRepository(),
      HiveSourcePreferenceRepository(),
    );
  }

  Future<void> close() async {
    await engineStateRepository.box.close();
    await documentRepository.box.close();
    await activeSearchRepository.box.close();
    await activeDocumentDataRepository.box.close();
    await sourceReactedRepository.box.close();
    await sourcePreferenceRepository.box.close();
  }
}

void main() {
  setupLogging();

  final source1 = Source('1.invalid.example');
  final source2 = Source('2.invalid.example');
  final source3 = Source('3.invalid.example');
  final source4 = Source('4.invalid.example');

  final docs = [
    Document(
      documentId: DocumentId(),
      stackId: StackId.fromString('311dc7eb-5fc7-4aa4-8232-e119f7e80e76'),
      batchIndex: 1,
      userReaction: UserReaction.positive,
      isActive: true,
      isSearched: false,
      resource: NewsResource(
        country: 'DE',
        language: 'de',
        datePublished: DateTime.utc(2020, 1, 7, 8, 9),
        image: Uri.parse('http://$source1/foo/imag.png'),
        rank: 1,
        score: 1,
        snippet: 'snippet1',
        sourceDomain: source1,
        title: 'title1',
        topic: 'topic1',
        url: Uri.parse('http://$source1/foo/index.html'),
      ),
    ),
    Document(
      documentId: DocumentId(),
      stackId: StackId.nil(),
      batchIndex: 2,
      userReaction: UserReaction.neutral,
      isActive: true,
      isSearched: true,
      resource: NewsResource(
        country: 'DE',
        language: 'de',
        datePublished: DateTime.utc(2020, 2, 7, 8, 9),
        image: Uri.parse('http://$source2/foo/imag.png'),
        rank: 2,
        score: 2,
        snippet: 'snippet2',
        sourceDomain: source2,
        title: 'title2',
        topic: 'topic2',
        url: Uri.parse('http://$source2/foo/index.html'),
      ),
    ),
    Document(
      documentId: DocumentId(),
      stackId: StackId.fromString('311dc7eb-5fc7-4aa4-8232-e119f7e80e76'),
      batchIndex: 3,
      userReaction: UserReaction.negative,
      isActive: false,
      isSearched: false,
      resource: NewsResource(
        country: 'US',
        language: 'en',
        datePublished: DateTime.utc(2020, 3, 7, 8, 9),
        image: Uri.parse('http://$source3/foo/imag.png'),
        rank: 3,
        score: 3,
        snippet: 'snippet3',
        sourceDomain: source3,
        title: 'title3',
        topic: 'topic3',
        url: Uri.parse('http://$source3/foo/index.html'),
      ),
    ),
    Document(
      documentId: DocumentId(),
      stackId: StackId.nil(),
      batchIndex: 4,
      userReaction: UserReaction.positive,
      isActive: false,
      isSearched: true,
      resource: NewsResource(
        country: 'US',
        language: 'en',
        datePublished: DateTime.utc(2020, 4, 7, 8, 9),
        image: Uri.parse('http://$source4/foo/imag.png'),
        rank: 4,
        score: 4,
        snippet: 'snippet4',
        sourceDomain: source4,
        title: 'title4',
        topic: 'topic4',
        url: Uri.parse('http://$source4/foo/index.html'),
      ),
    )
  ];

  group('Hive to Rust migration', () {
    late LocalNewsApiServer server;
    late TestEngineData data;
    DiscoveryEngine? engine;

    setUp(() async {
      data = await setupTestEngineData(useEphemeralDb: false);
      server = await LocalNewsApiServer.start();

      EventHandler.registerHiveAdapters();
      await EventHandler.initDatabase(data.applicationDirectoryPath);
    });

    tearDown(() async {
      await engine?.dispose();
      await server.close();
      await Directory(data.applicationDirectoryPath).delete(recursive: true);
    });

    test('works with mostly empty db (a)', () async {
      await HiveInjection.accessRepositories(data.applicationDirectoryPath,
          (repos) async {
        await repos.sourceReactedRepository.save(
          SourceReacted(source1, true),
        );
      });

      engine = await initEngine(data, server.port);

      await HiveInjection.accessRepositories(data.applicationDirectoryPath,
          (repos) async {
        expect(repos.engineStateRepository.isEmpty, isTrue);
        expect(repos.documentRepository.isEmpty, isTrue);
        expect(repos.activeSearchRepository.isEmpty, isTrue);
        expect(repos.activeDocumentDataRepository.isEmpty, isTrue);
        expect(repos.sourceReactedRepository.isEmpty, isTrue);
        expect(repos.sourcePreferenceRepository.isEmpty, isTrue);
      });
    });

    test('works with mostly empty db (b)', () async {
      await HiveInjection.accessRepositories(data.applicationDirectoryPath,
          (repos) async {
        await repos.documentRepository.update(docs.first);
      });

      engine = await initEngine(data, server.port);

      final feed =
          expectEvent<RestoreFeedSucceeded>(await engine!.restoreFeed()).items;
      expect(feed[0], equals(docs[0].toApiRepr()));
      expect(feed.length, equals(1));

      await HiveInjection.accessRepositories(data.applicationDirectoryPath,
          (repos) async {
        expect(repos.engineStateRepository.isEmpty, isTrue);
        expect(repos.documentRepository.isEmpty, isTrue);
        expect(repos.activeSearchRepository.isEmpty, isTrue);
        expect(repos.activeDocumentDataRepository.isEmpty, isTrue);
        expect(repos.sourceReactedRepository.isEmpty, isTrue);
        expect(repos.sourcePreferenceRepository.isEmpty, isTrue);
      });
    });

    test('works with needing migration for all data', () async {
      await HiveInjection.accessRepositories(data.applicationDirectoryPath,
          (repos) async {
        //FIXME test engine state migration by having a known valid engine state blob
        await repos.documentRepository.updateMany(docs);
        final data0 =
            ActiveDocumentData(Embedding(Float32List.fromList([2, 25, 2, 3])));
        data0.viewTime[DocumentViewMode.web] = const Duration(minutes: 2);
        await repos.activeDocumentDataRepository.update(
          docs[0].documentId,
          data0,
        );

        await repos.activeDocumentDataRepository.update(
          docs[1].documentId,
          ActiveDocumentData(Embedding(Float32List.fromList([22, 25, 32, 33]))),
        );

        final data2 =
            ActiveDocumentData(Embedding(Float32List.fromList([12, 15, 2, 3])));
        data2.viewTime[DocumentViewMode.reader] = const Duration(minutes: 22);
        await repos.activeDocumentDataRepository.update(
          docs[2].documentId,
          data2,
        );

        await repos.activeSearchRepository.save(
          ActiveSearch(
            searchBy: SearchBy.query,
            searchTerm: 'test',
            pageSize: 12,
            requestedPageNb: 21,
          ),
        );

        await repos.sourceReactedRepository.save(
          SourceReacted(source1, true),
        );
        await repos.sourceReactedRepository.save(
          SourceReacted(source3, false),
        );
        await repos.sourceReactedRepository.save(
          SourceReacted(source4, true),
        );

        await repos.sourcePreferenceRepository
            .save(SourcePreference.trusted(source1));
        await repos.sourcePreferenceRepository
            .save(SourcePreference.trusted(Source('foo.example')));
        await repos.sourcePreferenceRepository
            .save(SourcePreference.excluded(Source('bar.example')));
      });

      // should run migrations
      engine = await initEngine(data, server.port);
      await engine!.dispose();
      // should not run migrations again
      engine = await initEngine(data, server.port);

      expect(
        await engine!.getTrustedSourcesList(),
        equals(
          EngineEvent.trustedSourcesListRequestSucceeded({
            source1,
            Source('foo.example'),
          }),
        ),
      );

      expect(
        await engine!.getExcludedSourcesList(),
        equals(
          EngineEvent.excludedSourcesListRequestSucceeded({
            Source('bar.example'),
          }),
        ),
      );

      final feed =
          expectEvent<RestoreFeedSucceeded>(await engine!.restoreFeed()).items;
      expect(feed.first, equals(docs[0].toApiRepr()));
      expect(feed.length, equals(1));

      final search = expectEvent<RestoreActiveSearchSucceeded>(
        await engine!.restoreActiveSearch(),
      ).items;
      expect(search.first, equals(docs[1].toApiRepr()));
      expect(search.length, equals(1));

      await HiveInjection.accessRepositories(data.applicationDirectoryPath,
          (repos) async {
        expect(repos.engineStateRepository.isEmpty, isTrue);
        expect(repos.documentRepository.isEmpty, isTrue);
        expect(repos.activeSearchRepository.isEmpty, isTrue);
        expect(repos.activeDocumentDataRepository.isEmpty, isTrue);
        expect(repos.sourceReactedRepository.isEmpty, isTrue);
        expect(repos.sourcePreferenceRepository.isEmpty, isTrue);
      });
    });

    // ignore: require_trailing_commas
  }, skip: !cfgFeatureStorage);
}
