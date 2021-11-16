import 'dart:async' show Stream;
import 'dart:html' show DedicatedWorkerGlobalScope;

import 'package:xayn_discovery_engine/src/worker/common/platform_actors.dart'
    show PlatformWorker;

class _WebWorker extends PlatformWorker {
  DedicatedWorkerGlobalScope get _context =>
      DedicatedWorkerGlobalScope.instance;

  @override
  Stream<Object> get messages =>
      _context.onMessage.map((event) => event.data as Object);

  @override
  void send(Object message, [List<Object>? transfer]) =>
      _context.postMessage(message, transfer);

  @override
  void dispose() => _context.close();
}

PlatformWorker createPlatformWorker(Object initialMessage) => _WebWorker();
