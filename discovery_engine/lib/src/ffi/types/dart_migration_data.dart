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

import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustDartMigrationData;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart';
import 'package:xayn_discovery_engine/src/ffi/types/primitives.dart';
import 'package:xayn_discovery_engine/src/infrastructure/migration.dart';

extension DartMigrationDataFfi on DartMigrationData {
  Boxed<RustDartMigrationData> allocNative() {
    final place = ffi.alloc_uninitialized_dart_migration_data();
    writeNative(place);
    return Boxed(place, ffi.drop_dart_migration_data);
  }

  void writeNative(final Pointer<RustDartMigrationData> place) {
    engineState
        .writeNative(ffi.dart_migration_data_place_of_engine_state(place));
    //TODO[pmk] pass the actual data to rust and use it there
  }
}
