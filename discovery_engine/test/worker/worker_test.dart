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

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show
        ConverterException,
        Manager,
        ManagerDisposedException,
        ResponseTimeoutException;

import '../logging.dart' show setupLogging;
import 'mocks/managers.dart'
    show MockManager, ThrowsOnRequestManager, ThrowsOnResponseManager;
import 'mocks/workers.dart'
    show MockWorker, ThrowsOnRequestWorker, ThrowsOnResponseWorker;

void main() {
  setupLogging();

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
        manager.send({1: 'unexpected message'}),
        completion(equals('error')),
      );
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
      expect(
        manager.send('ping', timeout: Duration.zero),
        throwsA(isA<ResponseTimeoutException>()),
      );
    });

    test(
        'when sending a response that the Manager can NOT convert'
        'it should throw a `ResponseTimeoutException`', () async {
      manager = await MockManager.create(ThrowsOnResponseWorker.entryPoint);
      expect(
        manager.send('ping', timeout: Duration.zero),
        throwsA(isA<ResponseTimeoutException>()),
      );
    });
  });

  group('dispose method', () {
    test(
        'when disposing a manager it should close the responses stream '
        'and throw a "ManagerDisposedException" when trying to use '
        'the "send" method', () async {
      final manager = await MockManager.create(MockWorker.entryPoint);
      await manager.dispose();

      expect(
        manager.responses,
        emitsInOrder(<Object>[
          emitsDone,
        ]),
      );
      expect(
        manager.send(''),
        throwsA(isA<ManagerDisposedException>()),
      );
    });
  });
}
