import 'dart:isolate' show Isolate, ReceivePort, SendPort;

import 'package:xayn_discovery_engine/src/worker/common/platform_actors.dart'
    show PlatformManager;

typedef _EntryPoint = void Function(SendPort sendPort);

class _IsolatedPlatformManager extends PlatformManager {
  final Isolate _worker;
  final SendPort _workerChannel;
  final Stream _managerChannel;
  final Stream _errorChannel;

  _IsolatedPlatformManager._(
    this._managerChannel,
    this._worker,
    this._workerChannel,
    this._errorChannel,
  );

  static Future<PlatformManager> spawn(_EntryPoint entryPoint) async {
    final mainPort = ReceivePort();
    final errorPort = ReceivePort();
    final worker = await Isolate.spawn(
      entryPoint,
      mainPort.sendPort,
      errorsAreFatal: false,
      onError: errorPort.sendPort,
    );

    final managerChannel = mainPort.asBroadcastStream();
    final errorChannel = errorPort.asBroadcastStream();
    final workerChannel = await managerChannel.first as SendPort;

    return _IsolatedPlatformManager._(
        managerChannel, worker, workerChannel, errorChannel);
  }

  @override
  Stream get errors => _errorChannel;

  @override
  Stream get messages => _managerChannel;

  @override
  void send(dynamic message, [List<Object>? transfer]) =>
      _workerChannel.send(message);

  @override
  void dispose() => _worker.kill();
}

Future<PlatformManager> createPlatformManager(dynamic entryPoint) =>
    _IsolatedPlatformManager.spawn(entryPoint as _EntryPoint);
