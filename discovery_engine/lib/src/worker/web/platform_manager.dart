import 'dart:html' show Worker, ErrorEvent;

import 'package:xayn_discovery_engine/src/worker/common/exceptions.dart'
    show WorkerSpawnException;
import 'package:xayn_discovery_engine/src/worker/common/platform_actors.dart'
    show PlatformManager;

const kScriptUrl = 'worker.dart.js';

class WebWorkerManager extends PlatformManager {
  final Worker _worker;

  WebWorkerManager._(this._worker);

  static Future<PlatformManager> spawn(String scriptUrl) async {
    if (Worker.supported == false) {
      throw WorkerSpawnException(
        'WebWorkers are not supported in this browser',
      );
    }

    final worker = Worker(scriptUrl);
    return WebWorkerManager._(worker);
  }

  @override
  Stream<Object> get errors => _worker.onError.map<Object>((event) {
        final e = event as ErrorEvent;
        // TODO: check what would be the best format
        return [e.error, e.message ?? ''];
      });

  @override
  Stream<Object> get messages =>
      _worker.onMessage.map((event) => event.data as Object);

  @override
  void send(Object message, [List<Object>? transfer]) =>
      _worker.postMessage(message, transfer);

  @override
  void dispose() => _worker.terminate();
}

Future<PlatformManager> createPlatformManager(Object? scriptUrl) =>
    WebWorkerManager.spawn(scriptUrl as String? ?? kScriptUrl);
