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

import 'dart:convert' show Converter;

import 'package:xayn_discovery_engine/src/api/api.dart'
    show ClientEvent, EngineEvent;
import 'package:xayn_discovery_engine/src/api/codecs/json_codecs.dart'
    show OneshotRequestToJsonConverter, JsonToEngineEventConverter;
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show Manager, OneshotRequest, PlatformManager;

class DiscoveryEngineManager extends Manager<ClientEvent, EngineEvent> {
  final _requestConverter = OneshotRequestToJsonConverter();
  final _responseConverter = JsonToEngineEventConverter();

  DiscoveryEngineManager._(PlatformManager manager) : super(manager);

  static Future<DiscoveryEngineManager> create(Object? entryPoint) async {
    final platformManager = await Manager.spawnWorker(entryPoint);
    return DiscoveryEngineManager._(platformManager);
  }

  @override
  Converter<OneshotRequest<ClientEvent>, Object> get requestConverter =>
      _requestConverter;

  @override
  Converter<Object, EngineEvent> get responseConverter => _responseConverter;
}
