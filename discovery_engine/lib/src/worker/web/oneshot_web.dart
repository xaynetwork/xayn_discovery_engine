import 'dart:html' show MessageChannel, MessagePort;

import 'package:xayn_discovery_engine/src/worker/common/oneshot.dart'
    show OneshotChannel, SendingPort, ReceivingPort;

class WebSendingPort extends SendingPort {
  final MessagePort _port;
  WebSendingPort(this._port);

  @override
  void close() => _port.close();

  @override
  void send(dynamic message, [List<Object>? transfer]) =>
      _port.postMessage(message, transfer);
}

class WebReceivingPort extends ReceivingPort {
  final MessagePort _port;
  WebReceivingPort(this._port);

  @override
  void close() => _port.close();

  @override
  Future<Object?> receive() => _port.onMessage.first;
}

OneshotChannel createChannel() {
  final channel = MessageChannel();
  final sendingPort = WebSendingPort(channel.port1);
  final receivingPort = WebReceivingPort(channel.port2);
  return OneshotChannel(sendingPort, receivingPort);
}
