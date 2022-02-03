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
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/document/document.dart'
    show Document;

void main() {
  test('reading and written a document', () {
    final document = Document(
      id: DocumentId(),
      stackId: StackId(),
      rank: 12,
      title: 'Dodo Mania',
      snipped: 'Cloning bought back the dodo.',
      url: 'htts://foo.example/bar',
      domain: 'foo.example',
      smbertEmbedding: Float32List.fromList([.9, .1]),
    );
    final place = ffi.alloc_uninitialized_document();
    document.writeTo(place);
    final res = Document.readFrom(place);
    ffi.drop_document(place);
    expect(res, equals(document));
  });
}
