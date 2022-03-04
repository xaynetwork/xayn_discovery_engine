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

import 'dart:async' show FutureOr;
import 'dart:ffi' show NativeType, nullptr, Pointer;

class Boxed<RustType extends NativeType> {
  Pointer<RustType> _ptr;
  final FutureOr<void> Function(Pointer<RustType>) _free;

  /// Creates a new wrapper instance.
  ///
  /// Ptr must point to a non-dangling instance of `RustType`.
  Boxed(this._ptr, this._free);

  /// True if `free` or `move` was called.
  bool get moved => _ptr == nullptr;

  /// Returns the equivalent of an `&mut RustType`.
  ///
  /// While the returned pointer is used _anywhere_ you must not:
  ///
  /// - call mut
  /// - call ref
  /// - call free
  /// - call move
  Pointer<RustType> get mut {
    if (moved) {
      throw StateError('the pointer is no longer valid, either freed or moved');
    }
    return _ptr;
  }

  /// Returns the equivalent of an `&RustType`.
  ///
  /// While the returned pointer is used _anywhere_ you must not:
  ///
  /// - call mut
  /// - call free
  /// - call move
  Pointer<RustType> get ref {
    if (moved) {
      throw StateError('the pointer is no longer valid, either freed or moved');
    }
    return _ptr;
  }

  /// Frees/drops the boxed type, if it wasn't already dropped or moved.
  ///
  /// Is always safe to call if this type was constructed/used correctly.
  FutureOr<void> free() async {
    if (!moved) {
      await _free(move());
    }
  }

  /// Moves the instance out of this wrapper.
  Pointer<RustType> move() {
    if (moved) {
      throw StateError('the pointer is no longer valid, either freed or moved');
    }
    final res = _ptr;
    _ptr = nullptr;
    return res;
  }
}
