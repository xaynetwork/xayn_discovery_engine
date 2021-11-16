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
