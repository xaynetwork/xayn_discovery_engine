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
          'WebWorkers are not supported in this browser');
    }

    final worker = Worker(scriptUrl);
    return WebWorkerManager._(worker);
  }

  @override
  Stream get errors => _worker.onError.map<dynamic>((event) {
        final e = event as ErrorEvent;
        // TODO: check what would be the best format
        // => https://xainag.atlassian.net/browse/TY-2219
        return [e.error, e.message ?? ''];
      });

  @override
  Stream get messages => _worker.onMessage.map<dynamic>((event) => event.data);

  @override
  void send(dynamic message, [List<Object>? transfer]) =>
      _worker.postMessage(message, transfer);

  @override
  void dispose() => _worker.terminate();
}

Future<PlatformManager> createPlatformManager(dynamic scriptUrl) =>
    WebWorkerManager.spawn(scriptUrl as String? ?? kScriptUrl);
