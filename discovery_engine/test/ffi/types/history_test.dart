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

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/history.dart'
    show HistoricDocument;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/ffi/types/history.dart'
    show HistoricDocumentFfi, HistoricDocumentSliceFfi;

void main() {
  test('reading written HistoricDocument works', () {
    final document = HistoricDocument(
      id: DocumentId(),
      url: Uri.parse('https://www.test.test/foo'),
      snippet: 'foo, bar and foobar',
      title: 'Foobar',
    );
    final boxed = document.allocNative();
    final res = HistoricDocumentFfi.readNative(boxed.ref);
    boxed.free();
    expect(res, equals(document));
  });

  test('reading written Vec<HistoricDocument> works', () {
    final docs = [
      HistoricDocument(
        id: DocumentId(),
        url: Uri.parse('https://foo.example/'),
        snippet: 'foo',
        title: 'bar',
      ),
      HistoricDocument(
        id: DocumentId(),
        url: Uri.parse('https://foo.example/'),
        snippet: 'dodo',
        title: 'bird',
      )
    ];

    final boxed = docs.allocNative();
    final res = HistoricDocumentSliceFfi.consumeNative(boxed);
    expect(res, equals(docs));
  });
}
