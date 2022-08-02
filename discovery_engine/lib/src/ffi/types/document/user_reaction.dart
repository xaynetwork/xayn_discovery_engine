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

import 'dart:ffi' show Pointer, Uint8Pointer, nullptr;

import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show UserReaction, UserReactionIntConversion;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustOptionUserReaction, RustUserReaction1;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart';

extension UserReactionFfi on UserReaction {
  void writeNative(final Pointer<RustUserReaction1> place) {
    place.value = toIntRepr();
  }

  static UserReaction readNative(
    final Pointer<RustUserReaction1> place,
  ) =>
      UserReactionIntConversion.fromIntRepr(place.value);
}

extension OptionUserReactionFfi on UserReaction? {
  void writeNative(final Pointer<RustOptionUserReaction> place) {
    final repr = this?.toIntRepr();
    if (repr == null) {
      ffi.init_option_user_reaction_none_at(place);
    } else {
      if (ffi.init_option_user_reaction_some_at(place, repr) != 1) {
        throw ArgumentError(
          'dart UserReaction incompatible with rust UserReaction',
        );
      }
    }
  }

  static UserReaction? readNative(
    final Pointer<RustOptionUserReaction> place,
  ) {
    final pointer = ffi.get_option_user_reaction_some(place);
    if (pointer == nullptr) {
      return null;
    } else {
      return UserReactionFfi.readNative(pointer);
    }
  }
}
