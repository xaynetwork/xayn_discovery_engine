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

import 'dart:convert' show Converter;

import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show Worker, OneshotRequest;
import 'converters.dart'
    show
        OneshotToMessageConverter,
        MessageToOneshotConverter,
        DoesNothingConverter,
        OneshotToExceptionConverter,
        MessageToExceptionConverter;

final osToMsg = OneshotToMessageConverter();
final msgToOs = MessageToOneshotConverter();
final defaultConverter = DoesNothingConverter();
final osToException = OneshotToExceptionConverter();
final msgToException = MessageToExceptionConverter();

class MockWorker extends Worker<Object, Object> {
  MockWorker(Object initialMessage) : super(initialMessage);

  @override
  void onError(Object error, StackTrace stackTrace, {Object? incomingMessage}) {
    send('$error');
  }

  @override
  Future<void> onMessage(request) async {
    if (request.payload == 'ping') {
      send('pong', request.sender);
    } else if (request.payload is Map &&
        (request.payload as Map)['message'] != null) {
      send((request.payload as Map)['message'] as String, request.sender);
    } else {
      send('error', request.sender);
    }
  }

  @override
  Converter<Object, OneshotRequest<Object>> get requestConverter => msgToOs;

  @override
  Converter<Object, Object> get responseConverter => defaultConverter;

  static void entryPoint(Object initialMessage) => MockWorker(initialMessage);
}

class ThrowsOnRequestWorker extends MockWorker {
  ThrowsOnRequestWorker(Object initialMessage) : super(initialMessage);

  @override
  Converter<Object, OneshotRequest<Object>> get requestConverter =>
      msgToException;

  static void entryPoint(Object initialMessage) =>
      ThrowsOnRequestWorker(initialMessage);
}

class ThrowsOnResponseWorker extends MockWorker {
  ThrowsOnResponseWorker(Object initialMessage) : super(initialMessage);

  @override
  Converter<Object, Object> get responseConverter => osToException;

  static void entryPoint(Object initialMessage) =>
      ThrowsOnResponseWorker(initialMessage);
}
