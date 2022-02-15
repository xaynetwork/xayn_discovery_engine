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
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustUserReacted;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reaction.dart'
    show UserReactionFfi;
import 'package:xayn_discovery_engine/src/ffi/types/embedding.dart'
    show EmbeddingFfi;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show StringFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart'
    show DocumentIdFfi, StackIdFfi;

class UserReactedFfi with EquatableMixin {
  final DocumentId id;
  final StackId stackId;
  final String snippet;
  final Float32List smbertEmbedding;
  final UserReaction reaction;

  UserReactedFfi({
    required this.id,
    required this.stackId,
    required this.snippet,
    required this.smbertEmbedding,
    required this.reaction,
  });

  factory UserReactedFfi.readNative(final Pointer<RustUserReacted> place) {
    return UserReactedFfi(
      id: DocumentIdFfi.readNative(ffi.user_reacted_place_of_id(place)),
      stackId: StackIdFfi.readNative(ffi.user_reacted_place_of_stack_id(place)),
      snippet: StringFfi.readNative(ffi.user_reacted_place_of_snippet(place)),
      smbertEmbedding: EmbeddingFfi.readNative(
        ffi.user_reacted_place_of_smbert_embedding(place),
      ),
      reaction: UserReactionFfi.readNative(
        ffi.user_reacted_place_of_reaction(place),
      ),
    );
  }

  Boxed<RustUserReacted> allocNative() {
    final place = ffi.alloc_uninitialized_user_reacted();
    writeNative(place);
    return Boxed(place, ffi.drop_user_reacted);
  }

  void writeNative(final Pointer<RustUserReacted> place) {
    id.writeNative(ffi.user_reacted_place_of_id(place));
    stackId.writeNative(ffi.user_reacted_place_of_stack_id(place));
    snippet.writeNative(ffi.user_reacted_place_of_snippet(place));
    smbertEmbedding
        .writeNative(ffi.user_reacted_place_of_smbert_embedding(place));
    reaction.writeNative(ffi.user_reacted_place_of_reaction(place));
  }

  @override
  List<Object?> get props => [
        id,
        stackId,
        snippet,
        smbertEmbedding,
        reaction,
      ];
}
