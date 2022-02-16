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

import 'package:xayn_discovery_engine/src/domain/models/time_spent.dart'
    show TimeSpent;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustTimeSpent;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reaction.dart'
    show UserReactionFfi;
import 'package:xayn_discovery_engine/src/ffi/types/duration.dart'
    show DurationFfi;
import 'package:xayn_discovery_engine/src/ffi/types/embedding.dart'
    show EmbeddingFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart'
    show DocumentIdFfi;

extension TimeSpentFfi on TimeSpent {
  static TimeSpent readNative(final Pointer<RustTimeSpent> place) {
    return TimeSpent(
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

  Boxed<RustTimeSpent> allocNative() {
    final place = ffi.alloc_uninitialized_time_spend();
    writeNative(place);
    return Boxed(place, ffi.drop_time_spent);
  }

  void writeNative(final Pointer<RustTimeSpent> place) {
    id.writeNative(ffi.time_spent_place_of_id(place));
    smbertEmbedding
        .writeNative(ffi.time_spent_place_of_smbert_embedding(place));
    time.writeNative(ffi.time_spent_place_of_time(place));
    reaction.writeNative(ffi.time_spent_place_of_reaction(place));
  }
}
