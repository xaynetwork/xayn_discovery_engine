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

import 'dart:typed_data';

import 'package:hive/hive.dart';
import 'package:meta/meta.dart';

import 'package:xayn_discovery_engine/src/domain/assets/assets.dart';
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart';
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart';
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart';
import 'package:xayn_discovery_engine/src/domain/models/source.dart';
import 'package:xayn_discovery_engine/src/domain/models/source_preference.dart';
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart';
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_search_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_document_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_engine_state_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_source_preference_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_source_reacted_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_duration_adapter.dart';
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_embedding_adapter.dart';
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_source_adapter.dart';
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_unique_id_adapter.dart';
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_uri_adapter.dart';

bool _hiveRegistered = false;

class DartMigrationData {
  final Uint8List? engineState;
  final List<Document> documents;
  final List<SourceReacted> reactedSources;
  final Set<Source> trustedSources;
  final Set<Source> excludedSources;
  final ActiveSearch? activeSearch;
  final Map<DocumentId, ActiveDocumentData> activeDocumentData;

  DartMigrationData({
    required this.engineState,
    required this.documents,
    required this.activeDocumentData,
    required this.reactedSources,
    required this.trustedSources,
    required this.excludedSources,
    required this.activeSearch,
  });

  /// If migrations is necessary extracts the data for it from the repos.
  static Future<DartMigrationData?> fromDirectoryPath(
    String applicationDirectoryPath,
  ) async {
    registerHiveAdapters();
    await initDatabase(applicationDirectoryPath);
    final engineStateRepository = HiveEngineStateRepository();
    final documentRepository = HiveDocumentRepository();
    final activeSearchRepository = HiveActiveSearchRepository();
    final activeDocumentDataRepository = HiveActiveDocumentDataRepository();
    final sourceReactedRepository = HiveSourceReactedRepository();
    final sourcePreferenceRepository = HiveSourcePreferenceRepository();

    final dartMigrationData = (engineStateRepository.isEmpty &&
            documentRepository.isEmpty &&
            activeSearchRepository.isEmpty &&
            activeDocumentDataRepository.isEmpty &&
            sourceReactedRepository.box.isEmpty &&
            sourcePreferenceRepository.isEmpty)
        ? null
        : DartMigrationData(
            engineState: await engineStateRepository.load(),
            documents: await documentRepository.fetchAll(),
            activeDocumentData: activeDocumentDataRepository.box.toMap().map(
                  (Object? key, data) =>
                      MapEntry(DocumentId.fromString(key as String), data),
                ),
            reactedSources: await sourceReactedRepository.fetchAll(),
            trustedSources: await sourcePreferenceRepository.getTrusted(),
            excludedSources: await sourcePreferenceRepository.getExcluded(),
            activeSearch: await activeSearchRepository.getCurrent(),
          );

    await engineStateRepository.clear();
    await documentRepository.clear();
    await activeSearchRepository.clear();
    await activeDocumentDataRepository.clear();
    await sourceReactedRepository.clear();
    await sourcePreferenceRepository.clear();
    await Hive.close();

    return dartMigrationData;
  }
}

@visibleForTesting
void registerHiveAdapters() {
  if (_hiveRegistered) return;
  _hiveRegistered = true;
  Hive.registerAdapter(DocumentAdapter());
  Hive.registerAdapter(UserReactionAdapter());
  Hive.registerAdapter(DocumentViewModeAdapter());
  Hive.registerAdapter(ActiveDocumentDataAdapter());
  Hive.registerAdapter(NewsResourceAdapter());
  Hive.registerAdapter(DocumentIdAdapter());
  Hive.registerAdapter(StackIdAdapter());
  Hive.registerAdapter(DurationAdapter());
  Hive.registerAdapter(UriAdapter());
  Hive.registerAdapter(EmbeddingAdapter());
  Hive.registerAdapter(FeedMarketAdapter());
  Hive.registerAdapter(SearchByAdapter());
  Hive.registerAdapter(ActiveSearchAdapter());
  Hive.registerAdapter(SourceAdapter());
  Hive.registerAdapter(SetSourceAdapter());
  Hive.registerAdapter(SourcePreferenceAdapter());
  Hive.registerAdapter(PreferenceModeAdapter());
  Hive.registerAdapter(SourceReactedAdapter());
}

@visibleForTesting
Future<void> initDatabase(String appDir) async {
  Hive.init('$appDir/$kDatabasePath');

  // open boxes
  await Future.wait([
    _openDbBox<Document>(documentBox),
    _openDbBox<ActiveDocumentData>(activeDocumentDataBox),

    /// See TY-2799
    /// Hive usually compacts our boxes automatically. However, with the default
    /// strategy, compaction is triggered after 60 deleted entries. This leads
    /// to the problem that our engine state is constantly growing because we
    /// are only overwriting it and not deleting it. Therefore we call it with
    /// `compact: true`.
    _openDbBox<Uint8List>(engineStateBox, compact: true),
    _openDbBox<ActiveSearch>(searchBox),
    _openDbBox<Set<Source>>(trustedSourcesBox),
    _openDbBox<Set<Source>>(excludedSourcesBox),
    _openDbBox<SourcePreference>(sourcePreferenceBox),
    _openDbBox<SourceReacted>(sourceReactedBox),
  ]);
}

/// Tries to open a box persisted on disk. In case of failure opens it in memory.
/// If `compact` is set to `true`, compaction of the box will be triggered
/// after opening.
Future<void> _openDbBox<T>(String name, {bool compact = false}) async {
  try {
    final box = await Hive.openBox<T>(name);

    if (compact) {
      await box.compact();
    }
  } catch (e) {
    /// Some browsers (e.g. Firefox) are not allowing the use of IndexedDB
    /// in `Private Mode`, so we need to use Hive in-memory instead
    await Hive.openBox<T>(name, bytes: Uint8List(0));
  }
}
