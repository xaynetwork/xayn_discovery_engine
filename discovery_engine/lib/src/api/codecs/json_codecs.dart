import 'dart:convert' show Converter;

import 'package:xayn_discovery_engine/src/api/api.dart'
    show ClientEvent, EngineEvent;
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show ConverterException, OneshotRequest, Sender, SendingPort;

const kSenderKey = 'sender';
const kPayloadKey = 'payload';

class OneshotRequestToJsonConverter
    extends Converter<OneshotRequest<ClientEvent>, Map<String, Object>> {
  @override
  Map<String, Object> convert(OneshotRequest<ClientEvent> input) {
    try {
      return <String, Object>{
        kSenderKey: input.sender.platformPort,
        kPayloadKey: input.payload.toJson(),
      };
    } catch (e) {
      throw ConverterException(
        'OneshotRequest to JSON conversion failed',
        payload: input,
        source: e,
      );
    }
  }
}

class JsonToOneshotRequestConverter
    extends Converter<Map<String, Object>, OneshotRequest<ClientEvent>> {
  @override
  OneshotRequest<ClientEvent> convert(Map<String, Object> input) {
    try {
      final jsonPayload = (input[kPayloadKey] as Map).cast<String, Object>();
      final payload = ClientEvent.fromJson(jsonPayload);
      final sender = getSenderFromJson(input);
      return OneshotRequest(sender, payload);
    } catch (e) {
      throw ConverterException(
        'JSON to OneshotRequest conversion failed',
        payload: input,
        source: e,
      );
    }
  }

  Sender<SendingPort> getSenderFromJson(Map<String, Object> input) {
    final jsonSender = input[kSenderKey] as Object;
    return Sender.fromPlatformPort(jsonSender);
  }
}

class EngineEventToJsonConverter
    extends Converter<EngineEvent, Map<String, Object>> {
  @override
  Map<String, Object> convert(EngineEvent input) {
    try {
      return input.toJson().cast();
    } catch (e) {
      throw ConverterException(
        'EngineEvent to JSON conversion failed',
        payload: input,
        source: e,
      );
    }
  }
}

class JsonToEngineEventConverter
    extends Converter<Map<String, Object>, EngineEvent> {
  @override
  EngineEvent convert(Map<String, Object> input) {
    try {
      return EngineEvent.fromJson(input);
    } catch (e) {
      throw ConverterException(
        'JSON to EngineEvent conversion failed',
        payload: input,
        source: e,
      );
    }
  }
}
