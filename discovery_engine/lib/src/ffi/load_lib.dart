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

import 'dart:ffi' show DynamicLibrary;
import 'dart:io' show Platform;

import 'package:xayn_discovery_engine/src/ffi/genesis.ext.dart' show AsyncCore;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show XaynDiscoveryEngineBindingsFfi;

/// Opens the platform dependent Rust library.
DynamicLibrary _open() {
  if (Platform.isLinux) {
    return DynamicLibrary.open(
      '../discovery_engine_core/target/debug/libxayn_discovery_engine_bindings.so',
    );
  }
  if (Platform.isMacOS) {
    return DynamicLibrary.open(
      '../discovery_engine_core/target/debug/libxayn_discovery_engine_bindings.dylib',
    );
  }
  throw UnsupportedError('Unsupported platform.');
}

/// The handle to the C-FFI of the Rust library.
final ffi = XaynDiscoveryEngineBindingsFfi(_open());
final asyncCore = AsyncCore(ffi);
