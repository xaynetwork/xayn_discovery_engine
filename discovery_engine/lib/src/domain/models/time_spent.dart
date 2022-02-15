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

import 'dart:typed_data';

import 'package:freezed_annotation/freezed_annotation.dart' show freezed;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

part 'time_spent.freezed.dart';

/// TimeSpent event with metadata as passed to the engine.
@freezed
class TimeSpent with _$TimeSpent {
  const factory TimeSpent({
    required DocumentId id,
    required Float32List smbertEmbedding,
    required Duration time,
    required UserReaction reaction,
  }) = _TimeSpent;
}
