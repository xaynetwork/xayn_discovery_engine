import 'dart:isolate' show Isolate, ReceivePort, SendPort;

import 'package:xayn_discovery_engine/src/worker/common/platform_actors.dart'
    show PlatformManager;

typedef _EntryPoint = void Function(SendPort sendPort);

class _IsolatedPlatformManager extends PlatformManager {
  final Isolate _worker;
  final SendPort _workerChannel;
  final Stream<dynamic> _managerChannel;

  _IsolatedPlatformManager._(
    this._managerChannel,
    this._worker,
    this._workerChannel,
  );

  static Future<PlatformManager> spawn(_EntryPoint entryPoint) async {
    final channel = ReceivePort();
    final worker = await Isolate.spawn(
      entryPoint,
      channel.sendPort,
      errorsAreFatal: false,
    );
    // TODO: if we create a broadcast stream,
    // do we need to close the `ReceivePort`?
    final managerChannel = channel.asBroadcastStream();
    final workerChannel = await managerChannel.first as SendPort;
    return _IsolatedPlatformManager._(managerChannel, worker, workerChannel);
  }

  @override
  // TODO: do we need to clean-up stream subscriptions?
  Stream get messages => _managerChannel;

  @override
  void send(dynamic message, [List<Object>? transfer]) {
    _workerChannel.send(message);
  }

  @override
  void dispose() {
    _worker.kill();
    // TODO: is any other clean-up needed?
  }
}

Future<PlatformManager> createPlatformManager(dynamic entryPoint) =>
    _IsolatedPlatformManager.spawn(entryPoint as _EntryPoint);
