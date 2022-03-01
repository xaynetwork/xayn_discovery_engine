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

import 'package:hive/hive.dart' show Hive;
import 'package:xayn_discovery_engine/src/domain/assets/data_provider.dart'
    show kDatabasePath;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show engineStateBox;
import 'package:xayn_discovery_engine/src/infrastructure/repository/hive_engine_state_repo.dart'
    show HiveEngineStateRepository;

Future<Uint8List?> loadEngineState(String applicationDirectoryPath) async {
  Hive.init('$applicationDirectoryPath/$kDatabasePath');
  await Hive.openBox<Uint8List>(engineStateBox);
  final state = await HiveEngineStateRepository().load();
  await Hive.close();
  return state;
}

Future<void> saveEngineState(
  String applicationDirectoryPath,
  Uint8List state,
) async {
  Hive.init('$applicationDirectoryPath/$kDatabasePath');
  await Hive.openBox<Uint8List>(engineStateBox);
  await HiveEngineStateRepository().save(state);
  await Hive.close();
}
