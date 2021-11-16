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
  Future<Object?> receive() async => (await _port.onMessage.first).data;
}

OneshotChannel createChannel() {
  final channel = MessageChannel();
  final sendingPort = WebSendingPort(channel.port1);
  final receivingPort = WebReceivingPort(channel.port2);
  return OneshotChannel(sendingPort, receivingPort);
}

SendingPort createPlatformSendingPort(Object port) =>
    WebSendingPort(port as MessagePort);
