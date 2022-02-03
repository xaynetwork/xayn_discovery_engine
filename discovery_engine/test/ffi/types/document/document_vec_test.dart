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
import 'package:xayn_discovery_engine/src/ffi/types/document/document.dart';
import 'package:xayn_discovery_engine/src/ffi/types/document/document_vec.dart'
    show DocumentSliceFfi;

void main() {
  test('reading and writing a list of documents', () {
    final documents = <Document>[
      Document(
        id: DocumentId(),
        stackId: StackId(),
        rank: 12,
        title: 'Dodo Mania',
        snipped: 'Cloning bought back the dodo.',
        url: 'htts://foo.example/bar',
        domain: 'foo.example',
        smbertEmbedding: Float32List.fromList([.9, .1]),
      ),
      Document(
        id: DocumentId(),
        stackId: StackId(),
        rank: 1,
        title: 'Foobar',
        snipped: 'bar foo',
        url: 'htts://dodo.example/bird',
        domain: 'dodo.example',
        smbertEmbedding: Float32List.fromList([.1]),
      )
    ];
    final len = documents.length;
    final slice = documents.createSlice(len);
    final res = DocumentSliceFfi.readSlice(slice, len);
    ffi.drop_document_slice(slice, len);
    expect(res, equals(documents));
  });
}
