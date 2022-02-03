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

import 'dart:typed_data';

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reacted.dart'
    show UserReacted;

void main() {
  test('reading written user reacted instance yields same result', () {
    final document = UserReacted(
      id: DocumentId(),
      stackId: StackId(),
      snipped: 'Cloning bought back the dodo.',
      smbertEmbedding: Float32List.fromList([.9, .1]),
      feedback: DocumentFeedback.negative,
    );
    final place = ffi.alloc_uninitialized_user_reacted();
    document.writeTo(place);
    final res = UserReacted.readFrom(place);
    ffi.drop_user_reacted(place);
    expect(res, equals(document));
  });
}
