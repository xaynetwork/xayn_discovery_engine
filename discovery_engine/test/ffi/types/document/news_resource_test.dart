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
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResource;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/document/news_resource.dart'
    show NewsResourceFfi;

void main() {
  test('reading and written a document', () {
    final resource = NewsResource(
      title: 'fun',
      snippet: 'fun is fun',
      url: Uri.parse('https://www.foobar.example/dodo'),
      sourceDomain: 'www.example',
      image: null,
      datePublished: DateTime.now(),
      rank: 12,
      score: 32.25,
      country: 'Germany',
      language: 'German',
      topic: 'FunFun',
    );
    final place = ffi.alloc_uninitialized_news_resource();
    resource.writeNative(place);
    final res = NewsResourceFfi.readNative(place);
    ffi.drop_news_resource(place);
    expect(res, equals(resource));
  });
}
