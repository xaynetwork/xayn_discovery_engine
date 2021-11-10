import 'dart:convert';
import 'package:xayn_discovery_engine/src/worker/worker.dart';
import 'converters.dart';

final osToMsg = OneshotToMessageConverter();
final msgToOs = MessageToOneshotConverter();
final defaultConverter = DoesNothingConverter();
final osToException = OneshotToExceptionConverter();
final msgToException = MessageToExceptionConverter();

class MockWorker extends Worker<dynamic, dynamic> {
  MockWorker(dynamic initialMessage) : super(initialMessage);

  @override
  void onError(Object error, Emitter send) {
    send('$error');
  }

  @override
  void onMessage(OneshotRequest request, Emitter send) {
    if (request.payload == 'ping') {
      send('pong', request.sender);
    } else if (request.payload is Map && request.payload['message'] != null) {
      send(request.payload['message'], request.sender);
    } else {
      send('error', request.sender);
    }
  }

  @override
  Converter<dynamic, OneshotRequest> get requestConverter => msgToOs;

  @override
  Converter get responseConverter => defaultConverter;

  static void entryPoint(dynamic initialMessage) => MockWorker(initialMessage);
}

class ThrowsOnRequestWorker extends MockWorker {
  ThrowsOnRequestWorker(dynamic initialMessage) : super(initialMessage);

  @override
  Converter<dynamic, OneshotRequest> get requestConverter => msgToException;

  static void entryPoint(dynamic initialMessage) =>
      ThrowsOnRequestWorker(initialMessage);
}

class ThrowsOnResponseWorker extends MockWorker {
  ThrowsOnResponseWorker(dynamic initialMessage) : super(initialMessage);

  @override
  Converter get responseConverter => osToException;

  static void entryPoint(dynamic initialMessage) =>
      ThrowsOnResponseWorker(initialMessage);
}
