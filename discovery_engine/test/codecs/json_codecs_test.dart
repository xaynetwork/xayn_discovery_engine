import 'dart:isolate' show ReceivePort, SendPort;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/api/api.dart'
    show
        ClientEvent,
        DocumentId,
        DocumentFeedback,
        FeedRequested,
        FeedRequestSucceeded,
        ClientEventSucceeded,
        DocumentFeedbackChanged,
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

import 'matchers.dart' show throwsConverterException;
import 'mocks.dart' show BadClientEvent, BadEngineEvent;

void main() {
  group('OneshotRequestToJsonConverter', () {
    final converter = OneshotRequestToJsonConverter();
    late Oneshot channel;

    setUp(() {
      channel = Oneshot();
    });

    test(
        'when converting "FeedRequested" event, should contain a "SendPort" '
        'and a proper payload', () {
      const event_1 = ClientEvent.feedRequested();
      final request_1 = OneshotRequest(channel.sender, event_1);
      final message_1 = converter.convert(request_1) as Map;

      expect(message_1[kSenderKey], isA<SendPort>());
      expect(message_1[kPayloadKey], equals({'type': 'feedRequested'}));

      final documentId = DocumentId();
      final event_2 = ClientEvent.documentFeedbackChanged(
        documentId,
        DocumentFeedback.positive,
      );
      final request_2 = OneshotRequest(channel.sender, event_2);
      final message_2 = converter.convert(request_2) as Map;

      expect(message_2[kSenderKey], isA<SendPort>());
      expect(
        message_2[kPayloadKey],
        equals({
          'documentId': documentId.toJson(),
          'feedback': 1,
          'type': 'documentFeedbackChanged',
        }),
      );
    });

    test('when converting a "bad" event, should throw "ConverterException"',
        () {
      const event = BadClientEvent();
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
        kPayloadKey: {'type': 'feedRequested'}
      };
      final req_1 = converter.convert(event_1);

      expect(req_1.payload, isA<FeedRequested>());
      expect(req_1.sender, isA<Sender<SendingPort>>());
      expect(req_1.sender.platformPort, isA<SendPort>());
      expect(req_1.sender.platformPort, port);

      final event_2 = {
        kSenderKey: port,
        kPayloadKey: {
          'documentId': documentId.toJson(),
          'feedback': 1,
          'type': 'documentFeedbackChanged',
        }
      };

      final req_2 = converter.convert(event_2);
      // ignore: non_constant_identifier_names
      final req_2_payload = req_2.payload as DocumentFeedbackChanged;

      expect(req_2.payload, isA<DocumentFeedbackChanged>());
      expect(req_2_payload.documentId, documentId);
      expect(req_2_payload.feedback, DocumentFeedback.positive);
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
      const event_1 = EngineEvent.feedRequestSucceeded([]);
      final message_1 = converter.convert(event_1);

      expect(
        message_1,
        equals({'type': 'feedRequestSucceeded', 'items': <Object>[]}),
      );

      const event_2 = EngineEvent.engineExceptionRaised(
        EngineExceptionReason.noInitReceived,
      );
      final message_2 = converter.convert(event_2);

      expect(
        message_2,
        equals({'type': 'engineExceptionRaised', 'reason': 1}),
      );
    });

    test('when converting a "bad" event, should throw "ConverterException"',
        () {
      const event = BadEngineEvent();

      expect(() => converter.convert(event), throwsConverterException);
    });
  });

  group('JsonToEngineEventConverter', () {
    final converter = JsonToEngineEventConverter();

    test(
        'when converting correctly structured JSON Maps with events it should '
        'convert them to proper "EngineEvent" types', () {
      final event_1 = {
        'type': 'feedRequestSucceeded',
        'items': <Object>[],
      };
      final req_1 = converter.convert(event_1) as FeedRequestSucceeded;

      expect(req_1, isA<FeedRequestSucceeded>());
      expect(req_1.items, isEmpty);

      final event_2 = {
        'type': 'clientEventSucceeded',
      };
      final req_2 = converter.convert(event_2);

      expect(req_2, isA<ClientEventSucceeded>());
    });

    test('when converting a "bad" event, should throw "ConverterException"',
        () {
      expect(
        () => converter.convert({'type': 'feedRequested'}),
        throwsConverterException,
      );
      expect(
        () => converter.convert({'some': 'bad event'}),
        throwsConverterException,
      );
    });
  });
}
