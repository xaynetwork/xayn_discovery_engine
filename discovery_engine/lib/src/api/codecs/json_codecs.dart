import 'dart:convert' show Converter;

import 'package:xayn_discovery_engine/src/api/api.dart'
    show ClientEvent, EngineEvent;
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show OneshotRequest, Sender;

const kSenderKey = 'sender';
const kPayloadKey = 'payload';

class OneshotRequestToJsonConverter
    extends Converter<OneshotRequest<ClientEvent>, Map<String, Object>> {
  @override
  Map<String, Object> convert(OneshotRequest<ClientEvent> input) {
    return <String, Object>{
      kSenderKey: input.sender.platformPort,
      kPayloadKey: input.payload.toJson(),
    };
  }
}

class JsonToOneshotRequestConverter
    extends Converter<Map<String, Object>, OneshotRequest<ClientEvent>> {
  @override
  OneshotRequest<ClientEvent> convert(Map<String, Object> input) {
    final jsonSender = input[kSenderKey] as Object;
    final jsonPayload = (input[kPayloadKey] as Map).cast<String, Object>();
    final sender = Sender.fromPlatformPort(jsonSender);
    final payload = ClientEvent.fromJson(jsonPayload);
    return OneshotRequest(sender, payload);
  }
}

class EngineEventToJsonConverter
    extends Converter<EngineEvent, Map<String, Object>> {
  @override
  Map<String, Object> convert(EngineEvent input) {
    return input.toJson().cast();
  }
}

class JsonToEngineEventConverter
    extends Converter<Map<String, Object>, EngineEvent> {
  @override
  EngineEvent convert(Map<String, Object> input) {
    return EngineEvent.fromJson(input);
  }
}
