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

import 'package:hive/hive.dart' show Hive, Box;
import 'package:xayn_discovery_engine/src/domain/repository/engine_state_repo.dart'
    show EngineStateRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show engineStateBox;

/// Hive repository implementation of [EngineStateRepository].
class HiveEngineStateRepository implements EngineStateRepository {
  static const stateKey = 0;

  Box<Uint8List> get box => Hive.box<Uint8List>(engineStateBox);

  @override
  Future<Uint8List?> load() async => box.get(stateKey);

  @override
  Future<void> save(Uint8List bytes) => box.put(stateKey, bytes);

  @override
  Future<void> clear() async {
    await box.clear();
    await box.flush();
  }

  @override
  Future<bool> isEmpty() => Future.value(box.isEmpty);
}
