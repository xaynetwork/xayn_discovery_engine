import 'dart:convert' show Converter;

import 'package:xayn_discovery_engine/src/api/api.dart'
    show ClientEventGroups, EngineEventGroups;
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show OneshotRequest, Sender;

class OneshotRequestToJsonConverter
    extends Converter<OneshotRequest<ClientEventGroups>, Map<String, dynamic>> {
  @override
  Map<String, dynamic> convert(OneshotRequest<ClientEventGroups> input) {
    return <String, dynamic>{
      'sender': input.sender.platformPort,
      'payload': input.payload.toJson(),
    };
  }
}

class JsonToOneshotRequestConverter
    extends Converter<Map<String, dynamic>, OneshotRequest<ClientEventGroups>> {
  @override
  OneshotRequest<ClientEventGroups> convert(Map<String, dynamic> input) {
    final sender = Sender.fromPlatformPort(input['sender'] as Object);
    final payload = ClientEventGroups.fromJson(input);

    return OneshotRequest(sender, payload);
  }
}

class EngineEventGroupsToJsonConverter
    extends Converter<EngineEventGroups, Map<String, dynamic>> {
  @override
  Map<String, dynamic> convert(EngineEventGroups input) {
    return input.toJson();
  }
}

class JsonToEngineEventGroupsConverter
    extends Converter<Map<String, dynamic>, EngineEventGroups> {
  @override
  EngineEventGroups convert(Map<String, dynamic> input) {
    return EngineEventGroups.fromJson(input);
  }
}
