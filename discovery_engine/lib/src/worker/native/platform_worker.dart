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

import 'dart:async' show Stream;
import 'dart:isolate' show ReceivePort, SendPort;

import 'package:xayn_discovery_engine/src/worker/common/platform_actors.dart'
    show PlatformWorker;

class _IsolatedWorker extends PlatformWorker {
  final ReceivePort _workerChannel;
  final SendPort _managerChannel;

  _IsolatedWorker(SendPort sendPort)
      : _managerChannel = sendPort,
        _workerChannel = ReceivePort() {
    // send the isolate port as the first message
    _managerChannel.send(_workerChannel.sendPort);
  }

  @override
  Stream<Object> get messages =>
      _workerChannel.cast<Object>().asBroadcastStream();

  @override
  void send(Object message, [List<Object>? transfer]) =>
      _managerChannel.send(message);

  @override
  void dispose() {
    _workerChannel.close();
  }
}

PlatformWorker createPlatformWorker(Object initialMessage) =>
    _IsolatedWorker(initialMessage as SendPort);
