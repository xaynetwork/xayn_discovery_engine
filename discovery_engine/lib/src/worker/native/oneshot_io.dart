import 'dart:isolate' show ReceivePort, SendPort;

import 'package:xayn_discovery_engine/src/worker/common/oneshot.dart'
    show OneshotChannel, SendingPort, ReceivingPort;

class IsolatedSendingPort extends SendingPort {
  final SendPort _port;
  IsolatedSendingPort(this._port);

  @override
  void close() {
    // do nothing
  }

  @override
  void send(Object? message, [List<Object>? transfer]) => _port.send(message);
}

class IsolatedReceivingPort extends ReceivingPort {
  final ReceivePort _port;
  IsolatedReceivingPort(this._port);

  @override
  void close() => _port.close();

  @override
  Future<Object?> receive() => _port.first;
}

OneshotChannel createChannel() {
  final channel = ReceivePort();
  final sendingPort = IsolatedSendingPort(channel.sendPort);
  final receivingPort = IsolatedReceivingPort(channel);
  return OneshotChannel(sendingPort, receivingPort);
}
