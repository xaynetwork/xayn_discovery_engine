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

import 'dart:ffi' show NativeType, Pointer;

import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustDuration, RustOptionDuration;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;

extension DurationFfi on Duration {
  void writeNative(final Pointer<RustDuration> place) {
    writeWithWriter(ffi.init_duration_at, place);
  }

  /// Reads a dart [Duration] from a rust duration.
  ///
  /// Be aware that darts [Duration] has both less precision and  is more
  /// limited wrt. the max duration as it stores the duration as an `int`
  /// of microseconds.
  ///
  /// Sub microseconds precision will be ignored.
  ///
  /// In case of a too large durations an exception will be throw.
  static Duration readNative(final Pointer<RustDuration> place) =>
      _Helper.readWithReaders(
        ffi.get_duration_seconds,
        ffi.get_duration_nanos,
        place,
      );
}

extension OptionDurationFfi on Duration? {
  void writeNative(final Pointer<RustOptionDuration> place) {
    final self = this;
    if (self == null) {
      ffi.init_option_duration_none_at(place);
    } else {
      self.writeWithWriter(ffi.init_option_duration_some_at, place);
    }
  }

  /// Reads a dart [Duration] from an optional rust duration.
  ///
  /// Be aware that dart's [Duration] has both less precision and is more
  /// limited wrt. the max duration as it stores the duration as an `int`
  /// of microseconds.
  ///
  /// Sub microsecond precision will be ignored.
  ///
  /// In case of too large durations an exception will be thrown.
  static Duration? readNative(final Pointer<RustOptionDuration> place) {
    if (ffi.get_option_duration_is_some(place) == 0) {
      return null;
    } else {
      return _Helper.readWithReaders(
        ffi.get_option_duration_seconds,
        ffi.get_option_duration_nanos,
        place,
      );
    }
  }
}

extension _Helper on Duration {
  void writeWithWriter<T extends NativeType>(
    void Function(Pointer<T>, int, int) writer,
    Pointer<T> place,
  ) {
    final nanos = (inMicroseconds % Duration.microsecondsPerSecond) * 1000;
    writer(place, inSeconds, nanos);
  }

  static Duration readWithReaders<T extends NativeType>(
    int Function(Pointer<T>) readSeconds,
    int Function(Pointer<T>) readNanos,
    final Pointer<T> place,
  ) {
    const i64Max = 0x7fffffffffffffff;
    const maxDurationSeconds = i64Max ~/ Duration.microsecondsPerSecond;
    final seconds = readSeconds(place);
    if (seconds > maxDurationSeconds) {
      throw ArgumentError.value(
        seconds,
        'seconds',
        'duration is bigger than what dart Duration supports',
      );
    }
    final microseconds = readNanos(place) ~/ 1000;
    return Duration(seconds: seconds, microseconds: microseconds);
  }
}
