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
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResource;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustDocument;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/document/news_resource.dart'
    show NewsResourceFfi;
import 'package:xayn_discovery_engine/src/ffi/types/embedding.dart'
    show EmbeddingFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart'
    show DocumentIdFfi, StackIdFfi;

class DocumentFfi with EquatableMixin {
  final DocumentId id;
  final StackId stackId;
  final Float32List smbertEmbedding;
  final NewsResource resource;

  DocumentFfi({
    required this.id,
    required this.stackId,
    required this.smbertEmbedding,
    required this.resource,
  });

  @override
  List<Object?> get props => [id, stackId, smbertEmbedding, resource];

  factory DocumentFfi.readNative(final Pointer<RustDocument> place) {
    return DocumentFfi(
      id: DocumentIdFfi.readNative(ffi.document_place_of_id(place)),
      stackId: StackIdFfi.readNative(ffi.document_place_of_stack_id(place)),
      smbertEmbedding: EmbeddingFfi.readNative(
        ffi.document_place_of_smbert_embedding(place),
      ),
      resource:
          NewsResourceFfi.readNative(ffi.document_place_of_resource(place)),
    );
  }

  void writeNative(final Pointer<RustDocument> place) {
    id.writeNative(ffi.document_place_of_id(place));
    stackId.writeNative(ffi.document_place_of_stack_id(place));
    smbertEmbedding.writeNative(ffi.document_place_of_smbert_embedding(place));
    resource.writeNative(ffi.document_place_of_resource(place));
  }

  Document toDocument({required int batchIndex}) {
    return Document(
      documentId: id,
      stackId: stackId,
      resource: resource,
      batchIndex: batchIndex,
    );
  }
}
