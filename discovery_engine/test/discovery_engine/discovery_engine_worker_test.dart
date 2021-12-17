import 'dart:isolate' show ReceivePort;

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/api/api.dart'
    show
        EngineExceptionRaised,
        FeedRequestSucceeded,
        EngineExceptionReason,
        ClientEvent;
import 'package:xayn_discovery_engine/src/api/codecs/json_codecs.dart'
    show JsonToEngineEventConverter, kSenderKey, kPayloadKey;
import 'package:xayn_discovery_engine/src/discovery_engine_worker.dart'
    as entry_point show main, DiscoveryEngineWorker;
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show Manager, PlatformManager, OneshotRequest;

void main() {
  group('DiscoveryEngineWorker', () {
    late PlatformManager manager;
    late JsonToEngineEventConverter responseConverter;

    setUp(() async {
      manager = await Manager.spawnWorker(entry_point.main);
      responseConverter = JsonToEngineEventConverter();
    });

    tearDown(() {
      manager.dispose();
    });

    test(
        'when sending "FeedRequested" event as payload it should respond with '
        '"FeedRequestSucceeded" event', () async {
      final channel = ReceivePort();

      manager.send({
        kSenderKey: channel.sendPort,
        kPayloadKey: {'type': 'feedRequested'}
      });

      final responseMsg = await channel.first as Object;
      final response = responseConverter.convert(responseMsg);

      expect(response, isA<FeedRequestSucceeded>());
    });

    test(
        'when sending a bad massage it should respond with '
        '"EngineExceptionRaised" event with "converterException" reason',
        () async {
      manager.send('');

      final responseMsg = await manager.messages.first;
      final response = responseConverter.convert(responseMsg);

      expect(response, isA<EngineExceptionRaised>());
      expect(
        (response as EngineExceptionRaised).reason,
        EngineExceptionReason.converterException,
      );
    });

    test(
        'when sending a massage without a Sender it should respond with '
        '"EngineExceptionRaised" event with "converterException" reason',
        () async {
      manager.send({
        kPayloadKey: {'type': 'feedRequested'}
      });

      final responseMsg = await manager.messages.first;
      final response = responseConverter.convert(responseMsg);

      expect(response, isA<EngineExceptionRaised>());
      expect(
        (response as EngineExceptionRaised).reason,
        EngineExceptionReason.converterException,
      );
    });

    test(
        'when sending a message with a Sender but with bad payload it should '
        'respond with "EngineExceptionRaised" event with "converterException" '
        'reason but sent over using a Sender channel', () async {
      final channel = ReceivePort();

      manager.send({
        kSenderKey: channel.sendPort,
        kPayloadKey: {'bad': 'payload'}
      });

      final responseMsg = await channel.first as Object;
      final response = responseConverter.convert(responseMsg);

      expect(response, isA<EngineExceptionRaised>());
      expect(
        (response as EngineExceptionRaised).reason,
        EngineExceptionReason.converterException,
      );
    });

    test(
        'when the worker throws in the "onMessage" handler '
        'and we can not determine which exception was thrown it should respond '
        'with "EngineExceptionRaised" event with "genericError" reason',
        () async {
      // we need to use a worker that throws onMessage
      manager.dispose();
      manager = await Manager.spawnWorker(MockWorker.entryPoint);
      final channel = ReceivePort();

      manager.send({
        kSenderKey: channel.sendPort,
        kPayloadKey: {'type': 'feedRequested'}
      });

      final responseMsg = await channel.first as Object;
      final response = responseConverter.convert(responseMsg);

      expect(response, isA<EngineExceptionRaised>());
      expect(
        (response as EngineExceptionRaised).reason,
        EngineExceptionReason.genericError,
      );
    });
  });
}

class MockWorker extends entry_point.DiscoveryEngineWorker {
  MockWorker(Object message) : super(message);

  @override
  Future<void> onMessage(OneshotRequest<ClientEvent> request) async {
    await Future(() {
      throw Exception(' some random exception ');
    });
  }

  static void entryPoint(Object msg) => MockWorker(msg);
}
