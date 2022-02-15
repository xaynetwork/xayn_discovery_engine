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

import 'dart:typed_data' show Float32List;

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResource;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/document/document.dart'
    show DocumentFfi;

void main() {
  test('reading and written a document', () {
    final document = DocumentFfi(
      id: DocumentId(),
      stackId: StackId(),
      smbertEmbedding: Float32List.fromList([.9, .1]),
      resource: NewsResource(
        title: 'fun',
        snippet: 'fun is fun',
        url: Uri.parse('https://www.foobar.example/dodo'),
        sourceUrl: Uri.parse('yyy://www.example/'),
        thumbnail: null,
        datePublished: DateTime.now(),
        rank: 12,
        score: 32.5,
        country: 'Germany',
        language: 'German',
        topic: 'FunFun',
      ),
    );
    final place = ffi.alloc_uninitialized_document();
    document.writeNative(place);
    final res = DocumentFfi.readNative(place);
    ffi.drop_document(place);
    expect(res, equals(document));
  });

  test('conversion to Document works', () {
    final ffiDocument = DocumentFfi(
      id: DocumentId(),
      stackId: StackId(),
      smbertEmbedding: Float32List.fromList([.9, .1]),
      resource: NewsResource(
        title: 'fun',
        snippet: 'fun is fun',
        url: Uri.parse('https://www.foobar.example/dodo'),
        sourceUrl: Uri.parse('yyy://www.example/'),
        thumbnail: null,
        datePublished: DateTime.now(),
        rank: 12,
        score: 32.5,
        country: 'Germany',
        language: 'German',
        topic: 'FunFun',
      ),
    );
    final document = ffiDocument.toDocument(batchIndex: 12);
    expect(document.documentId, equals(ffiDocument.id));
    expect(document.stackId, equals(ffiDocument.stackId));
    expect(document.resource, equals(ffiDocument.resource));
    expect(document.batchIndex, equals(12));
    expect(document.userReaction, equals(UserReaction.neutral));
    expect(document.isActive, isTrue);
  });
}
