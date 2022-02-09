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
import 'package:xayn_discovery_engine/src/api/api.dart' show UserReaction;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reaction.dart'
    show UserReactionFfi;

void main() {
  test('reading written document feedback yields same result', () {
    for (final feedback in [
      UserReaction.neutral,
      UserReaction.positive,
      UserReaction.negative
    ]) {
      final place = ffi.alloc_uninitialized_user_reaction();
      feedback.writeNative(place);
      final res = UserReactionFfi.readNative(place);
      ffi.drop_user_reaction(place);
      expect(res, equals(feedback));
    }
  });
}
