import 'dart:convert' show Converter;

import 'package:xayn_discovery_engine/src/api/api.dart'
    show ClientEvent, EngineEvent;
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show OneshotRequest, Sender;

class OneshotRequestToJsonConverter
    extends Converter<OneshotRequest<ClientEvent>, Map<String, dynamic>> {
  @override
  Map<String, dynamic> convert(OneshotRequest<ClientEvent> input) {
    return <String, dynamic>{
      'sender': input.sender.platformPort,
      'payload': input.payload.toJson(),
    };
  }
}

class JsonToOneshotRequestConverter
    extends Converter<Map<String, dynamic>, OneshotRequest<ClientEvent>> {
  @override
  OneshotRequest<ClientEvent> convert(Map<String, dynamic> input) {
    final jsonSender = input['sender'] as Object;
    final jsonPayload = (input['payload'] as Map).cast<String, dynamic>();
    final sender = Sender.fromPlatformPort(jsonSender);
    final payload = ClientEvent.fromJson(jsonPayload);

    return OneshotRequest(sender, payload);
  }
}

class EngineEventToJsonConverter
    extends Converter<EngineEvent, Map<String, dynamic>> {
  @override
  Map<String, dynamic> convert(EngineEvent input) {
    return input.toJson();
  }
}

class JsonToEngineEventConverter
    extends Converter<Map<String, dynamic>, EngineEvent> {
  @override
  EngineEvent convert(Map<String, dynamic> input) {
    return EngineEvent.fromJson(input);
  }
}
