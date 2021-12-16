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
