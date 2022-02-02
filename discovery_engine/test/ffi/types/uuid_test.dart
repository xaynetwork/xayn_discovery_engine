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
import 'package:uuid/uuid.dart' show Uuid;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart'
    show DocumentIdFfi, StackIdFfi;

void main() {
  test('reading written document id yields same result', () {
    final uuid = DocumentId.fromBytes(Uuid.parseAsByteList(const Uuid().v4()));
    final place = ffi.alloc_uninitialized_uuid();
    uuid.writeNative(place);
    final res = DocumentIdFfi.readNative(place);
    ffi.drop_uuid(place);
    expect(res, equals(uuid));
  });

  test('reading written stack id yields same result', () {
    final uuid = StackId.fromBytes(Uuid.parseAsByteList(const Uuid().v4()));
    final place = ffi.alloc_uninitialized_uuid();
    uuid.writeNative(place);
    final res = StackIdFfi.readNative(place);
    ffi.drop_uuid(place);
    expect(res, equals(uuid));
  });
}
