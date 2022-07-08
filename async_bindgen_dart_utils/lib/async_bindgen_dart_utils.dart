// Copyright 2021 Xayn AG
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// Support for doing something awesome.
///
/// More dartdocs go here.
library easync_dart_io_utils;

import 'dart:async' show Completer, Future;
import 'dart:ffi' show NativePort;
import 'dart:isolate' show ReceivePort;

/// Magic tag send as additional failsafe, intentionally set to be not
/// precisely represented in JS.
// ignore:  avoid_js_rounded_ints
const int _magicTag = -6504203682518908873;

class CouldNotInitializeDartApiError extends Error {
  @override
  String toString() => 'Could not initialize dart api';
}

class FfiCompleterRegistry {
  static int _idGen = 0;
  static final _registry = <int, _FfiCompleter>{};
  static final _port = _setup();

  FfiCompleterRegistry._();

  static FfiSetup<T> newCompleter<T>({
    required T Function(int) extractor,
  }) {
    final completerId = _nextId();
    final ffiCompleter = _FfiCompleterImpl<T>(
      completer: Completer(),
      portId: _port.sendPort.nativePort,
      completerId: completerId,
      extractor: extractor,
    );
    _registry[completerId] = ffiCompleter;
    return ffiCompleter;
  }

  static int _nextId() {
    assert(_idGen < 0x1fffffffffffff);
    return _idGen++;
  }

  static ReceivePort _setup() {
    final port = ReceivePort('ffiCompleter');
    _startHandlingCompletions(port);
    return port;
  }

  static Future<void> _startHandlingCompletions(ReceivePort port) async {
    await for (final msg in port) {
      try {
        _handleMessage(msg);
      } catch (e) {
        //TODO log error
      }
    }
  }

  static void _handleMessage(Object? msg) {
    if (msg is List && msg[0] == _magicTag && msg.length >= 3) {
      assert(msg[1] is int);
      final completer = _takeCompleter(msg[1] as int);
      final dynamic result = msg[2];
      if (result is int) {
        completer.complete(result);
      } else if (result is String) {
        completer.completeError(FutureCanceled(result));
      } else {
        completer.completeError(
          FutureCanceled('unexpected result msg ${msg.toString()}'),
        );
      }
    } else {
      throw ArgumentError(
        'expected well formed async bindgen response, got: ${msg.toString()}',
      );
    }
  }

  static _FfiCompleter _takeCompleter(int id) {
    final completer = _registry.remove(id);
    if (completer == null) {
      throw StateError('no completer registered for completer id');
    }
    return completer;
  }
}

class FutureCanceled implements Exception {
  final String _msg;

  FutureCanceled(this._msg);

  @override
  String toString() => 'Rust Future was canceled: $_msg';
}

abstract class _FfiCompleter {
  void complete(int handle);
  void completeError(Object error);
}

abstract class FfiSetup<T> {
  int get portId;
  int get completerId;
  Future<T> get future;
}

class _FfiCompleterImpl<T> implements _FfiCompleter, FfiSetup<T> {
  T Function(int)? _extractor;
  final Completer<T> _completer;
  final int _portId;
  final int _completerId;

  _FfiCompleterImpl({
    required Completer<T> completer,
    required int portId,
    required int completerId,
    required T Function(int) extractor,
  })  : _completer = completer,
        _portId = portId,
        _completerId = completerId,
        _extractor = extractor;

  @override
  int get portId => _portId;
  @override
  int get completerId => _completerId;

  @override
  Future<T> get future => _completer.future;

  @override
  void complete(int handle) {
    final extractor = _extractor;
    if (extractor == null) {
      throw StateError('extractor was already used');
    }
    // while this method should never be called twice,
    // we still want to make sure the extractor is definitely
    // not called twice.
    _extractor = null;
    final val = extractor(handle);
    _completer.complete(val);
  }

  @override
  void completeError(Object error) {
    _extractor = null;
    _completer.completeError(error);
  }
}
