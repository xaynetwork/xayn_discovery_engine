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

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResource;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustDocument;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/document/news_resource.dart'
    show NewsResourceFfi;
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reaction.dart'
    show OptionUserReactionFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart'
    show DocumentIdFfi, StackIdFfi;

class DocumentFfi with EquatableMixin {
  final DocumentId id;
  final StackId stackId;
  final NewsResource resource;
  final UserReaction? reaction;

  DocumentFfi({
    required this.id,
    required this.stackId,
    required this.resource,
    this.reaction,
  });

  @override
  List<Object?> get props => [id, stackId, resource, reaction];

  factory DocumentFfi.readNative(final Pointer<RustDocument> place) {
    return DocumentFfi(
      id: DocumentIdFfi.readNative(ffi.document_place_of_id(place)),
      stackId: StackIdFfi.readNative(ffi.document_place_of_stack_id(place)),
      resource:
          NewsResourceFfi.readNative(ffi.document_place_of_resource(place)),
      reaction: OptionUserReactionFfi.readNative(
        ffi.document_place_of_reaction(place),
      ),
    );
  }

  void writeNative(final Pointer<RustDocument> place) {
    id.writeNative(ffi.document_place_of_id(place));
    stackId.writeNative(ffi.document_place_of_stack_id(place));
    resource.writeNative(ffi.document_place_of_resource(place));
    reaction.writeNative(ffi.document_place_of_reaction(place));
  }

  Document toDocument({bool isSearched = false}) => Document(
        documentId: id,
        stackId: stackId,
        resource: resource,
        batchIndex: 0, // unused
        isSearched: isSearched,
        userReaction: reaction ?? UserReaction.neutral,
      );

  ActiveDocumentData toActiveDocumentData() => ActiveDocumentData();
}
