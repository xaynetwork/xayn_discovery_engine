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

import 'dart:ffi' show Pointer;

import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show DocumentFeedback;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustUserReaction;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;

extension DocumentFeedbackFfi on DocumentFeedback {
  void writeNative(final Pointer<RustUserReaction> place) {
    var asInt = 128;
    switch (this) {
      case DocumentFeedback.neutral:
        asInt = 0;
        break;
      case DocumentFeedback.positive:
        asInt = 1;
        break;
      case DocumentFeedback.negative:
        asInt = 2;
        break;
    }
    final ok = ffi.init_user_reaction_at(place, asInt);
    if (ok == 0) {
      throw ArgumentError.value(
        this,
        'DocumentFeedback',
        'unsupported DocumentFeedback variant',
      );
    }
  }

  static DocumentFeedback readNative(
    final Pointer<RustUserReaction> place,
  ) {
    final reaction = ffi.get_user_reaction(place);
    switch (reaction) {
      case 0:
        return DocumentFeedback.neutral;
      case 1:
        return DocumentFeedback.positive;
      case 2:
        return DocumentFeedback.negative;
    }
    throw ArgumentError.value(
      reaction,
      'DocumentFeedback as int',
      'unexpected int representation',
    );
  }
}
