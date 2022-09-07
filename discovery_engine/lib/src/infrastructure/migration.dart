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

import 'package:xayn_discovery_engine/src/domain/models/active_data.dart';
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/source.dart';
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_document_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_active_search_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_document_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_engine_state_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_source_preference_repo.dart';
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_source_reacted_repo.dart';

class DartMigrationData {
  final Uint8List? engineState;
  final List<Document> documents;
  final Map<DocumentId, ActiveDocumentData> activeDocumentData;
  final List<SourceReacted> reactedSources;
  final Set<Source> trustedSources;
  final Set<Source> excludedSources;
  final ActiveSearch? activeSearch;

  /// Cleanup callback (which should be) called after successfully initializing the engine.
  final Future<void> Function() cleanup;

  DartMigrationData({
    required this.engineState,
    required this.documents,
    required this.activeDocumentData,
    required this.reactedSources,
    required this.trustedSources,
    required this.excludedSources,
    required this.activeSearch,
    required this.cleanup,
  });

  /// If migrations is necessary extracts the data for it from the repos.
  ///
  /// The `cleanup` callback is set to clear the repositories.
  static Future<DartMigrationData?> fromRepositories(
    HiveEngineStateRepository engineStateRepository,
    HiveDocumentRepository documentRepository,
    HiveActiveSearchRepository activeSearchRepository,
    HiveActiveDocumentDataRepository activeDocumentDataRepository,
    HiveSourceReactedRepository sourceReactedRepository,
    HiveSourcePreferenceRepository sourcePreferenceRepository,
  ) async {
    if (engineStateRepository.box.isEmpty &&
        documentRepository.box.isEmpty &&
        activeSearchRepository.box.isEmpty &&
        activeDocumentDataRepository.box.isEmpty &&
        //FIXME even with cfgFeatureStorage we still set this values to hive
        // sourcePreferenceRepository.box.isEmpty &&
        sourceReactedRepository.box.isEmpty) {
      return null;
    }

    return DartMigrationData(
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
      cleanup: () {
        //TODO[pmk] uncomment section once migration part was added
        // await engineStateRepository.clear();
        // await documentRepository.box.clear();
        // await activeSearchRepository.clear();
        // await activeDocumentDataRepository.box.clear();
        // await sourceReactedRepository.box.clear();
        // await sourcePreferenceRepository.clear();
        return Future<void>.value(null);
      },
    );
  }
}
