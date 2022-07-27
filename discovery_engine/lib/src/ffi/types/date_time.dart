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

import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustNaiveDateTime;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;

extension NaiveDateTimeFfi on DateTime {
  void writeNative(Pointer<RustNaiveDateTime> place) {
    // micro seconds since since since midnight on
    // January 1, 1970 ignoring time zone.
    ffi.init_naive_date_time_at(
      place,
      microsecondsSinceEpoch,
    );
  }

  static DateTime readNative(Pointer<RustNaiveDateTime> naiveDateTime) {
    final microsecondsSinceEpochLocal =
        ffi.get_naive_date_time_micros_since_epoch(naiveDateTime);
    final dateTime = DateTime.fromMicrosecondsSinceEpoch(
      microsecondsSinceEpochLocal,
      isUtc: true,
    );
    return dateTime;
  }
}
