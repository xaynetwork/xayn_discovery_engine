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

import 'dart:ffi' show Pointer, Uint64Pointer;
import 'dart:typed_data' show Float32List;

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustDocument;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/embedding.dart'
    show EmbeddingFfi;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show StringFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart'
    show DocumentIdFfi, StackIdFfi;

//FIXME dart Document model and rust Document are not at all in sync
//  once in sync we could use an extension block
class Document with EquatableMixin {
  final DocumentId id;
  final StackId stackId;
  final int rank;
  final String title;
  final String snipped;
  final String url;
  final String domain;
  final Float32List smbertEmbedding;

  Document({
    required this.id,
    required this.stackId,
    required this.rank,
    required this.title,
    required this.snipped,
    required this.url,
    required this.domain,
    required this.smbertEmbedding,
  });

  factory Document.readFrom(final Pointer<RustDocument> place) {
    return Document(
      id: DocumentIdFfi.readNative(ffi.document_place_of_id(place)),
      stackId: StackIdFfi.readNative(ffi.document_place_of_stack_id(place)),
      rank: ffi.document_place_of_rank(place).value,
      title: StringFfi.readNative(ffi.document_place_of_title(place)),
      snipped: StringFfi.readNative(ffi.document_place_of_snipped(place)),
      url: StringFfi.readNative(ffi.document_place_of_url(place)),
      domain: StringFfi.readNative(ffi.document_place_of_domain(place)),
      smbertEmbedding: EmbeddingFfi.readNative(
        ffi.document_place_of_smbert_embedding(place),
      ),
    );
  }

  void writeTo(final Pointer<RustDocument> place) {
    id.writeNative(ffi.document_place_of_id(place));
    stackId.writeNative(ffi.document_place_of_stack_id(place));
    ffi.document_place_of_rank(place).value = rank;
    title.writeNative(ffi.document_place_of_title(place));
    snipped.writeNative(ffi.document_place_of_snipped(place));
    url.writeNative(ffi.document_place_of_url(place));
    domain.writeNative(ffi.document_place_of_domain(place));
    smbertEmbedding.writeNative(ffi.document_place_of_smbert_embedding(place));
  }

  @override
  List<Object?> get props => [
        id,
        stackId,
        rank,
        title,
        snipped,
        url,
        domain,
        smbertEmbedding,
      ];
}
