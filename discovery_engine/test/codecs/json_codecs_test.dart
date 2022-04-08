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

import 'dart:isolate' show ReceivePort, SendPort;

import 'package:mockito/annotations.dart';
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart';
import 'package:xayn_discovery_engine/src/api/api.dart'
    show
        ClientEvent,
        DocumentId,
        UserReaction,
        RestoreFeedRequested,
        RestoreFeedSucceeded,
        ClientEventSucceeded,
        UserReactionChanged,
        EngineEvent,
        EngineExceptionReason;
import 'package:xayn_discovery_engine/src/api/codecs/json_codecs.dart'
    show
        EngineEventToJsonConverter,
        JsonToEngineEventConverter,
        JsonToOneshotRequestConverter,
        OneshotRequestToJsonConverter,
        kPayloadKey,
        kSenderKey;
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show Oneshot, OneshotRequest, Sender, SendingPort;

import '../logging.dart' show setupLogging;
import 'json_codecs_test.mocks.dart';
import 'matchers.dart' show throwsConverterException;

@GenerateMocks(
  [],
  customMocks: [
    MockSpec<ClientEvent>(
      unsupportedMembers: {#when, #maybeWhen, #map, #maybeMap},
    ),
    MockSpec<EngineEvent>(
      unsupportedMembers: {#when, #maybeWhen, #map, #maybeMap},
    ),
  ],
)
void main() {
  setupLogging();

  group('OneshotRequestToJsonConverter', () {
    final converter = OneshotRequestToJsonConverter();
    late Oneshot channel;

    setUp(() {
      channel = Oneshot();
    });

    test(
        'when converting "RestoreFeedRequested" event, should contain a "SendPort" '
        'and a proper payload', () {
      const event_1 = ClientEvent.restoreFeedRequested();
      final request_1 = OneshotRequest(channel.sender, event_1);
      final message_1 = converter.convert(request_1) as Map;

      expect(message_1[kSenderKey], isA<SendPort>());
      expect(
        message_1[kPayloadKey],
        equals({'runtimeType': 'restoreFeedRequested'}),
      );

      final documentId = DocumentId();
      final event_2 = ClientEvent.userReactionChanged(
        documentId,
        UserReaction.positive,
      );
      final request_2 = OneshotRequest(channel.sender, event_2);
      final message_2 = converter.convert(request_2) as Map;

      expect(message_2[kSenderKey], isA<SendPort>());
      expect(
        message_2[kPayloadKey],
        equals({
          'documentId': documentId.toJson(),
          'userReaction': 1,
          'runtimeType': 'userReactionChanged',
        }),
      );
    });

    test('when converting a "bad" event, should throw "ConverterException"',
        () {
      final event = MockClientEvent();
      final request = OneshotRequest(channel.sender, event);

      expect(() => converter.convert(request), throwsConverterException);
    });
  });

  group('JsonToOneshotRequestConverter', () {
    final converter = JsonToOneshotRequestConverter();
    late ReceivePort channel;

    setUp(() {
      channel = ReceivePort();
    });

    test(
        'when converting correctly structured requests it should convert them'
        'to proper "OneshotRequest" types with a "Sender" and "ClientEvent"',
        () {
      final port = channel.sendPort;
      final documentId = DocumentId();
      final event_1 = {
        kSenderKey: port,
        kPayloadKey: {'runtimeType': 'restoreFeedRequested'}
      };
      final req_1 = converter.convert(event_1);

      expect(req_1.payload, isA<RestoreFeedRequested>());
      expect(req_1.sender, isA<Sender<SendingPort>>());
      expect(req_1.sender.platformPort, isA<SendPort>());
      expect(req_1.sender.platformPort, port);

      final event_2 = {
        kSenderKey: port,
        kPayloadKey: {
          'documentId': documentId.toJson(),
          'userReaction': 1,
          'runtimeType': 'userReactionChanged',
        }
      };

      final req_2 = converter.convert(event_2);
      // ignore: non_constant_identifier_names
      final req_2_payload = req_2.payload as UserReactionChanged;

      expect(req_2.payload, isA<UserReactionChanged>());
      expect(req_2_payload.documentId, documentId);
      expect(req_2_payload.userReaction, UserReaction.positive);
      expect(req_2.sender, isA<Sender<SendingPort>>());
      expect(req_2.sender.platformPort, isA<SendPort>());
      expect(req_2.sender.platformPort, port);
    });

    test('when converting a "bad" event, should throw "ConverterException"',
        () {
      expect(
        () => converter.convert({'some': 'bad event'}),
        throwsConverterException,
      );
    });
  });

  group('EngineEventToJsonConverter', () {
    final converter = EngineEventToJsonConverter();

    test(
        'when converting "EngineEvent" types it should convert them'
        'to correctly structured JSON Maps', () {
      const event_1 = EngineEvent.restoreFeedSucceeded([]);
      final message_1 = converter.convert(event_1);

      expect(
        message_1,
        equals({'runtimeType': 'restoreFeedSucceeded', 'items': <Object>[]}),
      );

      const event_2 = EngineEvent.engineExceptionRaised(
        EngineExceptionReason.engineNotReady,
      );
      final message_2 = converter.convert(event_2);

      expect(
        message_2,
        equals({
          'runtimeType': 'engineExceptionRaised',
          'reason': 1,
          'message': null,
          'stackTrace': null,
        }),
      );
    });

    test('when converting a "bad" event, should throw "ConverterException"',
        () {
      final event = MockEngineEvent();

      expect(() => converter.convert(event), throwsConverterException);
    });
  });

  group('JsonToEngineEventConverter', () {
    final converter = JsonToEngineEventConverter();

    test(
        'when converting correctly structured JSON Maps with events it should '
        'convert them to proper "EngineEvent" types', () {
      final event_1 = {
        'runtimeType': 'restoreFeedSucceeded',
        'items': <Object>[],
      };
      final req_1 = converter.convert(event_1) as RestoreFeedSucceeded;

      expect(req_1, isA<RestoreFeedSucceeded>());
      expect(req_1.items, isEmpty);

      final event_2 = {
        'runtimeType': 'clientEventSucceeded',
      };
      final req_2 = converter.convert(event_2);

      expect(req_2, isA<ClientEventSucceeded>());
    });

    test('when converting a "bad" event, should throw "ConverterException"',
        () {
      expect(
        // a JSON representation of a [ClientEvent], not an [EngineEvent]
        () => converter.convert({'runtimeType': 'restoreFeedRequested'}),
        throwsConverterException,
      );
      expect(
        () => converter.convert({'some': 'bad event'}),
        throwsConverterException,
      );
    });
  });
}
