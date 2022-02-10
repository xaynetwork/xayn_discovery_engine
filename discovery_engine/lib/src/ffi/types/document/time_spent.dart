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

import 'dart:ffi' show Pointer;
import 'dart:typed_data' show Float32List;

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show DocumentId, UserReaction;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustTimeSpent;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reaction.dart'
    show UserReactionFfi;
import 'package:xayn_discovery_engine/src/ffi/types/duration.dart'
    show DurationFfi;
import 'package:xayn_discovery_engine/src/ffi/types/embedding.dart'
    show EmbeddingFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart'
    show DocumentIdFfi;

class TimeSpentFfi with EquatableMixin {
  final DocumentId id;
  final Float32List smbertEmbedding;
  final Duration time;
  final UserReaction reaction;

  TimeSpentFfi({
    required this.id,
    required this.smbertEmbedding,
    required this.time,
    required this.reaction,
  });

  factory TimeSpentFfi.readFrom(final Pointer<RustTimeSpent> place) {
    return TimeSpentFfi(
      id: DocumentIdFfi.readNative(ffi.time_spent_place_of_id(place)),
      smbertEmbedding: EmbeddingFfi.readNative(
        ffi.time_spent_place_of_smbert_embedding(place),
      ),
      time: DurationFfi.readNative(ffi.time_spent_place_of_time(place)),
      reaction: UserReactionFfi.readNative(
        ffi.time_spent_place_of_reaction(place),
      ),
    );
  }

  void writeTo(final Pointer<RustTimeSpent> place) {
    id.writeNative(ffi.time_spent_place_of_id(place));
    smbertEmbedding
        .writeNative(ffi.time_spent_place_of_smbert_embedding(place));
    time.writeNative(ffi.time_spent_place_of_time(place));
    reaction.writeNative(ffi.time_spent_place_of_reaction(place));
  }

  @override
  List<Object?> get props => [
        id,
        smbertEmbedding,
        time,
        reaction,
      ];
}
