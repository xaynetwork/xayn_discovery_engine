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
    show RustUrl, RustOptionUrl;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show BoxedStr;

extension UriFfi on Uri {
  void writeNative(Pointer<RustUrl> place) {
    final str = BoxedStr.create(toString());
    final ok = ffi.init_url_at(place, str.ptr, str.len);
    str.free();
    if (ok != 1) {
      throw ArgumentError('dart Uri incompatible with rust Url');
    }
  }

  static Uri readNative(Pointer<RustUrl> url) {
    final string = BoxedStr.fromRawParts(
      ffi.get_url_buffer(url),
      ffi.get_url_buffer_len(url),
    ).readNative();

    return Uri.parse(string);
  }

  static void writeNativeOption(Uri? self, Pointer<RustOptionUrl> place) {
    if (self == null) {
      ffi.inti_none_url_at(place);
    } else {
      final str = BoxedStr.create(self.toString());
      final ok = ffi.init_some_url_at(place, str.ptr, str.len);
      str.free();
      if (ok != 1) {
        throw ArgumentError('dart Uri incompatible with rust Url');
      }
    }
  }

  static Uri? readNativeOption(Pointer<RustOptionUrl> optUrl) {
    final url = ffi.get_option_url_some(optUrl);
    if (url.address == 0) {
      return null;
    } else {
      return UriFfi.readNative(url);
    }
  }
}
