// Copyright 2021 Xayn AG
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
    show RustDuration;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;

extension DurationFfi on Duration {
  void writeNative(final Pointer<RustDuration> durationPlace) {
    final nanos = (inMicroseconds % Duration.microsecondsPerSecond) * 1000;
    ffi.init_duration_at(durationPlace, inSeconds, nanos);
  }

  /// Reads a dart [Duration] from a rust duration.
  ///
  /// Be aware that darts [Duration] has both less precision and  is more
  /// limited wrt. the max duration as it stores the duration as a `int`
  /// of microseconds.
  ///
  /// Sub microseconds precision will be ignored.
  ///
  /// In case of to large durations an exception will be throw.
  ///
  static Duration readNative(final Pointer<RustDuration> durationPlace) {
    const maxDurationSeconds = 0x8637bd05af6;
    final seconds = ffi.get_duration_seconds(durationPlace);
    if (seconds > maxDurationSeconds) {
      throw ArgumentError.value(
        seconds,
        'seconds',
        'duration is bigger then what dart Duration supports',
      );
    }
    final microseconds = ffi.get_duration_nanos(durationPlace) ~/ 1000;
    return Duration(seconds: seconds, microseconds: microseconds);
  }
}
