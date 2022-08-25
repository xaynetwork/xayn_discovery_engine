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

import 'package:meta/meta.dart' show visibleForTesting;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustUuid, RustVecUuid;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/list.dart'
    show ListFfiAdapter;
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart'
    show DocumentIdFfi;

final _adapter = ListFfiAdapter<DocumentId, RustUuid, RustVecUuid>(
  alloc: ffi.alloc_uninitialized_uuid_slice,
  next: ffi.next_uuid,
  writeNative: (id, place) => id.writeNative(place),
  readNative: DocumentIdFfi.readNative,
  getVecLen: ffi.get_uuid_vec_len,
  getVecBuffer: ffi.get_uuid_vec_buffer,
  writeNativeVec: ffi.init_uuid_vec_at,
);

extension DocumentIdSetFfi on Set<DocumentId> {
  Boxed<RustVecUuid> allocNative() {
    final place = ffi.alloc_uninitialized_uuid_vec();
    _adapter.writeVec(toList(), place);
    return Boxed(place, ffi.drop_uuid_vec);
  }

  @visibleForTesting
  static Set<DocumentId> consumeNative(Boxed<RustVecUuid> boxed) {
    final result = _adapter.readVec(boxed.ref);
    boxed.free();
    return result.toSet();
  }
}
