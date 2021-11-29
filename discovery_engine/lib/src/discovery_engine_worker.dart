import 'dart:convert' show Converter;

import 'package:xayn_discovery_engine/src/api/api.dart'
    show ClientEventGroups, EngineEventGroups;
import 'package:xayn_discovery_engine/src/api/codecs/json_codecs.dart'
    show JsonToOneshotRequestConverter, EngineEventGroupsToJsonConverter;
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show Worker, OneshotRequest;

class DiscoveryEngineWorker
    extends Worker<ClientEventGroups, EngineEventGroups> {
  final _requestConverter = JsonToOneshotRequestConverter();
  final _responseConverter = EngineEventGroupsToJsonConverter();

  @override
  Converter<Map<String, dynamic>, OneshotRequest<ClientEventGroups>>
      get requestConverter => _requestConverter;

  @override
  Converter<EngineEventGroups, Map<String, dynamic>> get responseConverter =>
      _responseConverter;

  DiscoveryEngineWorker(Object message) : super(message);

  @override
  void onError(Object error) {
    // send('$error');
  }

  @override
  void onMessage(request) {
    //
  }
}

/// This method acts as an entry point:
/// - for Isolate.spawn on native platform
/// - for the compiled web worker file
void main(Object message) => DiscoveryEngineWorker(message);
