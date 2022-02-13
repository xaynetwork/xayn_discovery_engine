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

class ListFfiAdapter<T, RT extends NativeType, RVT extends NativeType> {
  final Pointer<RT> Function(int) alloc;
  final Pointer<RT> Function(Pointer<RT>) next;
  final void Function(T, Pointer<RT>) writeNative;
  final T Function(Pointer<RT>) readNative;
  final int Function(Pointer<RVT>) getVecLen;
  final Pointer<RT> Function(Pointer<RVT>) getVecBuffer;
  final void Function(Pointer<RVT>, Pointer<RT>, int) writeNativeVec;

  ListFfiAdapter({
    required this.alloc,
    required this.next,
    required this.writeNative,
    required this.readNative,
    required this.getVecLen,
    required this.getVecBuffer,
    required this.writeNativeVec,
  });

  /// Allocates a slice of markets containing all markets of this list.
  Pointer<RT> createSlice(List<T> list) {
    final slice = alloc(list.length);
    list.fold<Pointer<RT>>(slice, (nextElement, market) {
      writeNative(market, nextElement);
      return next(nextElement);
    });
    return slice;
  }

  /// Reads a rust-`&[T]` returning a dart-`List<T>`.
  List<T> readSlice(
    final Pointer<RT> ptr,
    final int len,
  ) {
    final out = <T>[];
    Iterable<int>.generate(len).fold<Pointer<RT>>(ptr, (nextElement, _) {
      out.add(readNative(nextElement));
      return next(nextElement);
    });
    return out;
  }

  /// Writes a `Vec<T>` to given place.
  void writeVec(
    final List<T> list,
    final Pointer<RVT> place,
  ) {
    final slice = createSlice(list);
    writeNativeVec(place, slice, list.length);
  }

  /// Reads a rust-`&Vec<T>` returning a dart-`List<T>`.
  List<T> readVec(
    final Pointer<RVT> vec,
  ) {
    final len = getVecLen(vec);
    final slice = getVecBuffer(vec);
    return readSlice(slice, len);
  }
}
