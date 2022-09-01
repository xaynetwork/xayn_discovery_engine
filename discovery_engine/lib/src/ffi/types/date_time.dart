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
    show RustDateTimeUtc;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;

extension DateTimeUtcFfi on DateTime {
  void writeNative(Pointer<RustDateTimeUtc> place) =>
      ffi.init_date_time_utc_at(place, microsecondsSinceEpoch);

  static DateTime readNative(Pointer<RustDateTimeUtc> dateTimeUtc) =>
      DateTime.fromMicrosecondsSinceEpoch(
        ffi.get_date_time_utc_micros_since_epoch(dateTimeUtc),
        isUtc: true,
      );
}
