// Copyright 2021 Xayn AG
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

import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:hive/hive.dart'
    show HiveType, HiveField, TypeAdapter, BinaryReader, BinaryWriter;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show documentViewModeTypeId;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustViewMode;

part 'view_mode.g.dart';

/// Document viewer mode.
@HiveType(typeId: documentViewModeTypeId)
@JsonEnum(alwaysCreate: true)
enum DocumentViewMode {
  @HiveField(RustViewMode.Story)
  @JsonValue(RustViewMode.Story)
  story,
  @HiveField(RustViewMode.Reader)
  @JsonValue(RustViewMode.Reader)
  reader,
  @HiveField(RustViewMode.Web)
  @JsonValue(RustViewMode.Web)
  web,
}

extension DocumentViewModeIntConversion on DocumentViewMode {
  int toIntRepr() => _$DocumentViewModeEnumMap[this]!;
  static DocumentViewMode fromIntRepr(int intRepr) =>
      $enumDecode(_$DocumentViewModeEnumMap, intRepr);
}
