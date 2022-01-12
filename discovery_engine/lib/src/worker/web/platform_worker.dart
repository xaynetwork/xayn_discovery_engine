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
