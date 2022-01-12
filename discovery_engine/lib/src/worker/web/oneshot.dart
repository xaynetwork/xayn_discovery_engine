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

import 'dart:html' show MessageChannel, MessagePort;

import 'package:xayn_discovery_engine/src/worker/common/oneshot.dart'
    show OneshotChannel, SendingPort, ReceivingPort;

class WebSendingPort extends SendingPort {
  final MessagePort _port;
  WebSendingPort(this._port);

  @override
  MessagePort get port => _port;

  @override
  void close() => _port.close();

  @override
  void send(Object message) => _port.postMessage(message);
}

class WebReceivingPort extends ReceivingPort {
  final MessagePort _port;
  WebReceivingPort(this._port);

  @override
  void close() => _port.close();

  @override
  Future<Object> receive() async =>
      (await _port.onMessage.first).data as Object;
}

OneshotChannel createChannel() {
  final channel = MessageChannel();
  final sendingPort = WebSendingPort(channel.port1);
  final receivingPort = WebReceivingPort(channel.port2);
  return OneshotChannel(sendingPort, receivingPort);
}

SendingPort createPlatformSendingPort(Object port) =>
    WebSendingPort(port as MessagePort);
