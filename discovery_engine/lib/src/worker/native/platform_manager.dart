import 'dart:isolate' show Isolate, ReceivePort, SendPort;

import 'package:xayn_discovery_engine/src/worker/common/platform_actors.dart'
    show PlatformManager;

typedef _EntryPoint = void Function(SendPort sendPort);

class _IsolatedPlatformManager extends PlatformManager {
  final Isolate _worker;
  final ReceivePort _mainPort;
  final ReceivePort _errorPort;
  final Stream _messages;
  final Stream _errors;
  final SendPort _workerChannel;

  _IsolatedPlatformManager._(
    this._worker,
    this._mainPort,
    this._errorPort,
    this._messages,
    this._errors,
    this._workerChannel,
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

    final messages = mainPort.asBroadcastStream();
    final errors = errorPort.asBroadcastStream();
    final workerChannel = await messages.first as SendPort;

    return _IsolatedPlatformManager._(
      worker,
      mainPort,
      errorPort,
      messages,
      errors,
      workerChannel,
    );
  }

  @override
  Stream<Object> get errors => _errors.cast<Object>();

  @override
  Stream<Object> get messages => _messages.cast<Object>();

  @override
  void send(Object message, [List<Object>? transfer]) =>
      _workerChannel.send(message);

  @override
  void dispose() {
    _mainPort.close();
    _errorPort.close();
    _worker.kill();
  }
}

Future<PlatformManager> createPlatformManager(Object? entryPoint) =>
    _IsolatedPlatformManager.spawn(entryPoint as _EntryPoint);
