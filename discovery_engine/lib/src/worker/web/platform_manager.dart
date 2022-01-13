// Copyright 2021 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
        // this is to align with messages that come from Isolate error stream
        return [e.message ?? e.error ?? 'Unknown error occured', ''];
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
