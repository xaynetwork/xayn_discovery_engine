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

import 'dart:ffi';
import 'dart:typed_data';

import 'package:xayn_discovery_engine/src/domain/models/active_data.dart';
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart';
import 'package:xayn_discovery_engine/src/domain/models/history.dart';
import 'package:xayn_discovery_engine/src/domain/models/source.dart';
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustDartMigrationData;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart';

class DartMigrationData {
  final Uint8List? engineState;
  final List<HistoricDocument> history;
  final Map<DocumentId, ActiveDocumentData> activeDocumentData;
  final List<SourceReacted> reactedSources;
  final Set<Source> trustedSources;
  final Set<Source> excludedSources;
  final ActiveSearch? activeSearch;

  DartMigrationData({
    required this.engineState,
    required this.history,
    required this.activeDocumentData,
    required this.reactedSources,
    required this.trustedSources,
    required this.excludedSources,
    required this.activeSearch,
  });

  Boxed<RustDartMigrationData> allocNative() {
    final place = ffi.alloc_uninitialized_dart_migration_data();
    writeNative(place);
    return Boxed(place, ffi.drop_dart_migration_data);
  }

  void writeNative(final Pointer<RustDartMigrationData> data) {
    //TODO[pmk] pass the actual data to rust and use it there
    ffi.dart_migration_data_place_of_dummy(data).value = 42;
  }
}
