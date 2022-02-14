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

class Boxed<RT extends NativeType> {
  Pointer<RT> _ptr;
  final void Function(Pointer<RT>) _free;

  /// Creates a new wrapper instance.
  ///
  /// Ptr must point to a non-dangling instance of `RT`.
  Boxed(this._ptr, this._free);

  /// True if `free` or `move` was called.
  bool get moved => _ptr.address == 0;

  /// Returns the equivalent of an `&mut RT`.
  ///
  /// While the returned pointer is used _anywhere_ you must not:
  ///
  /// - call mut
  /// - call ref
  /// - call free
  /// - call move
  Pointer<RT> get mut {
    if (moved) {
      throw StateError('the pointer is no longer valid, either freed or moved');
    }
    return _ptr;
  }

  /// Returns the equivalent of an `&RT`.
  ///
  /// While the returned pointer is used _anywhere_ you must not:
  ///
  /// - call mut
  /// - call free
  /// - call move
  Pointer<RT> get ref {
    if (moved) {
      throw StateError('the pointer is no longer valid, either freed or moved');
    }
    return _ptr;
  }

  /// Frees/drops the boxed type, if it wasn't already dropped or moved.
  ///
  /// Is always safe to call if this type was constructed/used correctly.
  void free() {
    if (!moved) {
      _free(move());
    }
  }

  /// Moves the instance out of this wrapper.
  Pointer<RT> move() {
    if (moved) {
      throw StateError('the pointer is no longer valid, either freed or moved');
    }
    final res = _ptr;
    _ptr = Pointer.fromAddress(0);
    return res;
  }
}
