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

import 'package:xayn_discovery_engine/src/domain/models/user_reacted.dart'
    show UserReacted;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustUserReacted;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reaction.dart'
    show UserReactionFfi;
import 'package:xayn_discovery_engine/src/ffi/types/embedding.dart'
    show EmbeddingFfi;
import 'package:xayn_discovery_engine/src/ffi/types/feed_market.dart'
    show FeedMarketFfi;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show StringFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart'
    show DocumentIdFfi, StackIdFfi;

extension UserReactedFfi on UserReacted {
  static UserReacted readNative(final Pointer<RustUserReacted> place) {
    return UserReacted(
      id: DocumentIdFfi.readNative(ffi.user_reacted_place_of_id(place)),
      stackId: StackIdFfi.readNative(ffi.user_reacted_place_of_stack_id(place)),
      title: StringFfi.readNative(ffi.user_reacted_place_of_title(place)),
      snippet: StringFfi.readNative(ffi.user_reacted_place_of_snippet(place)),
      smbertEmbedding: EmbeddingFfi.readNative(
        ffi.user_reacted_place_of_smbert_embedding(place),
      ),
      reaction: UserReactionFfi.readNative(
        ffi.user_reacted_place_of_reaction(place),
      ),
      market: FeedMarketFfi.readNative(ffi.user_reacted_place_of_market(place)),
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
    title.writeNative(ffi.user_reacted_place_of_title(place));
    snippet.writeNative(ffi.user_reacted_place_of_snippet(place));
    smbertEmbedding
        .writeNative(ffi.user_reacted_place_of_smbert_embedding(place));
    reaction.writeNative(ffi.user_reacted_place_of_reaction(place));
    market.writeNative(ffi.user_reacted_place_of_market(place));
  }
}
