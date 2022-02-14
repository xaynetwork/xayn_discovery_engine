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

class ListFfiAdapter<Type, RustType extends NativeType,
    RustVecType extends NativeType> {
  final Pointer<RustType> Function(int) alloc;
  final Pointer<RustType> Function(Pointer<RustType>) next;
  final void Function(Type, Pointer<RustType>) writeNative;
  final Type Function(Pointer<RustType>) readNative;
  final int Function(Pointer<RustVecType>) getVecLen;
  final Pointer<RustType> Function(Pointer<RustVecType>) getVecBuffer;
  final void Function(Pointer<RustVecType>, Pointer<RustType>, int)
      writeNativeVec;

  ListFfiAdapter({
    required this.alloc,
    required this.next,
    required this.writeNative,
    required this.readNative,
    required this.getVecLen,
    required this.getVecBuffer,
    required this.writeNativeVec,
  });

  /// Allocates a slice of `RustType` containing all items of this list.
  Pointer<RustType> createSlice(List<Type> list) {
    final slice = alloc(list.length);
    list.fold<Pointer<RustType>>(slice, (nextElement, market) {
      writeNative(market, nextElement);
      return next(nextElement);
    });
    return slice;
  }

  /// Reads a rust-`&[RustType]` returning a dart-`List<Type>`.
  List<Type> readSlice(
    final Pointer<RustType> ptr,
    final int len,
  ) {
    final out = <Type>[];
    Iterable<int>.generate(len).fold<Pointer<RustType>>(ptr, (nextElement, _) {
      out.add(readNative(nextElement));
      return next(nextElement);
    });
    return out;
  }

  /// Writes a `Vec<RustType>` to given place.
  void writeVec(
    final List<Type> list,
    final Pointer<RustVecType> place,
  ) {
    final slice = createSlice(list);
    writeNativeVec(place, slice, list.length);
  }

  /// Reads a rust-`&Vec<RustType>` returning a dart-`List<Type>`.
  List<Type> readVec(
    final Pointer<RustVecType> vec,
  ) {
    final len = getVecLen(vec);
    final slice = getVecBuffer(vec);
    return readSlice(slice, len);
  }
}
