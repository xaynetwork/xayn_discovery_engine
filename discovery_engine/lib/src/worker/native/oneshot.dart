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

import 'dart:isolate' show ReceivePort, SendPort;

import 'package:xayn_discovery_engine/src/worker/common/oneshot.dart'
    show OneshotChannel, SendingPort, ReceivingPort;

class IsolatedSendingPort extends SendingPort {
  final SendPort _port;
  IsolatedSendingPort(this._port);

  @override
  SendPort get port => _port;

  @override
  void close() {
    // [SendPort] can't close itself, it is closed by the controlling
    // [ReceivePort], so this method should do nothing
  }

  @override
  void send(Object message) => _port.send(message);
}

class IsolatedReceivingPort extends ReceivingPort {
  final ReceivePort _port;
  IsolatedReceivingPort(this._port);

  @override
  void close() => _port.close();

  @override
  Future<Object> receive() async => (await _port.first) as Object;
}

OneshotChannel createChannel() {
  final channel = ReceivePort();
  final sendingPort = IsolatedSendingPort(channel.sendPort);
  final receivingPort = IsolatedReceivingPort(channel);
  return OneshotChannel(sendingPort, receivingPort);
}

SendingPort createPlatformSendingPort(Object port) =>
    IsolatedSendingPort(port as SendPort);
