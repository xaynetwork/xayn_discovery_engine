import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/worker/worker.dart';

import 'mocks/managers.dart';
import 'mocks/workers.dart';

void main() {
  group('Worker abstraction:', () {
    late Manager manager;

    setUp(() async {
      manager = await MockManager.create(MockWorker.entryPoint);
    });

    tearDown(() async {
      await manager.dispose();
    });

    test(
        'when sending a message that the Worker can handle'
        'expect a corresponding response', () {
      expect(manager.send('ping'), completion(equals('pong')));
      expect(manager.send({'message': 'pong'}), completion(equals('pong')));
    });

    test(
        'when sending a message that the Worker can NOT handle'
        'expect a corresponding response', () {
      expect(manager.send('unexpected message'), completion(equals('error')));
      expect(
          manager.send({1: 'unexpected message'}), completion(equals('error')));
    });
  });

  group('Manager\'s converter throws on message convertion:', () {
    late Manager manager;

    tearDown(() async {
      await manager.dispose();
    });

    test(
        'when sending a request that the manager can NOT convert'
        'it should throw a `ConverterException`', () async {
      manager = await ThrowsOnRequestManager.create(MockWorker.entryPoint);
      expect(manager.send('ping'), throwsA(isA<ConverterException>()));
    });

    test(
        'when receiving a response that the manager can NOT convert'
        'it should throw a `ConverterException`', () async {
      manager = await ThrowsOnResponseManager.create(MockWorker.entryPoint);
      expect(manager.send('ping'), throwsA(isA<ConverterException>()));
    });
  });

  group('Worker\'s converter throws on message convertion:', () {
    late Manager manager;

    tearDown(() async {
      await manager.dispose();
    });

    test(
        'when receiving a request that the worker can NOT convert'
        'it should throw a `ResponseTimeoutException`', () async {
      manager = await MockManager.create(ThrowsOnRequestWorker.entryPoint);
      expect(manager.send('ping', timeout: Duration.zero),
          throwsA(isA<ResponseTimeoutException>()));
    });

    test(
        'when sending a response that the Manager can NOT convert'
        'it should throw a `ResponseTimeoutException`', () async {
      manager = await MockManager.create(ThrowsOnResponseWorker.entryPoint);
      expect(manager.send('ping', timeout: Duration.zero),
          throwsA(isA<ResponseTimeoutException>()));
    });
  });
}
