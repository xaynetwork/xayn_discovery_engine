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
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show ConverterException, OneshotRequest, Sender, SendingPort;

const kSenderKey = 'sender';
const kPayloadKey = 'payload';

class OneshotRequestToJsonConverter
    extends Converter<OneshotRequest<ClientEvent>, Object> {
  @override
  Object convert(OneshotRequest<ClientEvent> input) {
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
    extends Converter<Object, OneshotRequest<ClientEvent>> {
  @override
  OneshotRequest<ClientEvent> convert(Object input) {
    try {
      final map = (input as Map).cast<String, Object>();
      final jsonPayload = (map[kPayloadKey] as Map).cast<String, Object>();
      final payload = ClientEvent.fromJson(jsonPayload);
      final sender = getSenderFromJson(map);
      return OneshotRequest(sender, payload);
    } catch (e) {
      throw ConverterException(
        'JSON to OneshotRequest conversion failed',
        payload: input,
        source: e,
      );
    }
  }

  Sender<SendingPort> getSenderFromJson(Object input) {
    final map = (input as Map).cast<String, Object>();
    final jsonSender = map[kSenderKey] as Object;
    return Sender.fromPlatformPort(jsonSender);
  }
}

class EngineEventToJsonConverter extends Converter<EngineEvent, Object> {
  @override
  Object convert(EngineEvent input) {
    try {
      return input.toJson();
    } catch (e) {
      throw ConverterException(
        'EngineEvent to JSON conversion failed',
        payload: input,
        source: e,
      );
    }
  }
}

class JsonToEngineEventConverter extends Converter<Object, EngineEvent> {
  @override
  EngineEvent convert(Object input) {
    try {
      final map = (input as Map).cast<String, Object>();
      return EngineEvent.fromJson(map);
    } catch (e) {
      throw ConverterException(
        'JSON to EngineEvent conversion failed',
        payload: input,
        source: e,
      );
    }
  }
}
