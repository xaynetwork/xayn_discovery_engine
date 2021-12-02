import 'dart:convert' show Converter;

import 'package:xayn_discovery_engine/src/api/api.dart'
    show ClientEvent, EngineEvent;
import 'package:xayn_discovery_engine/src/api/codecs/json_codecs.dart'
    show OneshotRequestToJsonConverter, JsonToEngineEventConverter;
import 'package:xayn_discovery_engine/src/discovery_engine_worker.dart'
    as entry_point show main;
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show Manager, OneshotRequest, PlatformManager;

/// A constant that is true if the application was compiled to run on the web.
/// Taken from https://api.flutter.dev/flutter/foundation/kIsWeb-constant.html
const bool kIsWeb = identical(0, 0.0);

class DiscoveryEngineManager extends Manager<ClientEvent, EngineEvent> {
  final _requestConverter = OneshotRequestToJsonConverter();
  final _responseConverter = JsonToEngineEventConverter();

  DiscoveryEngineManager._(PlatformManager manager) : super(manager);

  static Future<DiscoveryEngineManager> create() async {
    final platformManager =
        await Manager.spawnWorker(kIsWeb ? null : entry_point.main);
    return DiscoveryEngineManager._(platformManager);
  }

  @override
  Converter<OneshotRequest<ClientEvent>, Map<String, dynamic>>
      get requestConverter => _requestConverter;

  @override
  Converter<Map<String, dynamic>, EngineEvent> get responseConverter =>
      _responseConverter;
}
