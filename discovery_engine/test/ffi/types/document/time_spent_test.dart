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
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/document/time_spent.dart'
    show TimeSpentFfi;

void main() {
  test('reading written user reacted instance yields same result', () {
    final timeSpent = TimeSpentFfi(
      id: DocumentId(),
      smbertEmbedding: Float32List.fromList([.9, .1]),
      time: const Duration(days: 2),
      reaction: UserReaction.negative,
    );
    final place = ffi.alloc_uninitialized_time_spend();
    timeSpent.writeTo(place);
    final res = TimeSpentFfi.readFrom(place);
    ffi.drop_time_spent(place);
    expect(res, equals(timeSpent));
  });
}
