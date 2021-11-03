import 'dart:html' show Worker;

import 'package:xayn_discovery_engine/src/worker/common/platform_actors.dart'
    show PlatformManager;

const kScriptUrl = 'worker.dart.js';

class WebWorkerManager extends PlatformManager {
  final Worker _worker;

  WebWorkerManager._(this._worker);

  static Future<PlatformManager> spawn(String scriptUrl) async {
    if (Worker.supported == false) {
      // TODO: maybe spawn a "SameThreadWorker" in such case
      throw UnsupportedError('Web workers are not supported');
    }

    final worker = Worker(scriptUrl);
    return WebWorkerManager._(worker);
  }

  @override
  Stream get messages => _worker.onMessage
      // TODO: do we need to clean-up stream subscriptions?
      .map<dynamic>((event) => event.data);

  @override
  void send(dynamic message, [List<Object>? transfer]) {
    _worker.postMessage(message, transfer);
  }

  @override
  void dispose() {
    _worker.terminate();
    // TODO: is any other clean-up needed?
  }
}

Future<PlatformManager> createPlatformManager(dynamic scriptUrl) =>
    WebWorkerManager.spawn(scriptUrl as String? ?? kScriptUrl);
