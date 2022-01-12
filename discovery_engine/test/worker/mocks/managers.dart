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

import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show Manager, PlatformManager, OneshotRequest;

import 'converters.dart'
    show
        OneshotToMessageConverter,
        MessageToExceptionConverter,
        MessageToOneshotConverter,
        DoesNothingConverter,
        OneshotToExceptionConverter;

final osToMsg = OneshotToMessageConverter();
final msgToOs = MessageToOneshotConverter();
final defaultConverter = DoesNothingConverter();
final osToException = OneshotToExceptionConverter();
final msgToException = MessageToExceptionConverter();

class MockManager extends Manager<Object, Object> {
  MockManager._(PlatformManager manager) : super(manager);

  @override
  Converter<OneshotRequest<Object>, Object> get requestConverter => osToMsg;

  @override
  Converter<Object, Object> get responseConverter => defaultConverter;

  static Future<MockManager> create(Object entryPoint) async {
    final platformManager = await Manager.spawnWorker(entryPoint);
    return MockManager._(platformManager);
  }
}

class ThrowsOnRequestManager extends Manager<Object, Object> {
  ThrowsOnRequestManager._(PlatformManager manager) : super(manager);

  @override
  Converter<OneshotRequest<Object>, Object> get requestConverter =>
      osToException;

  @override
  Converter<Object, Object> get responseConverter => defaultConverter;

  static Future<ThrowsOnRequestManager> create(Object entryPoint) async {
    final platformManager = await Manager.spawnWorker(entryPoint);
    return ThrowsOnRequestManager._(platformManager);
  }
}

class ThrowsOnResponseManager extends Manager<Object, Object> {
  ThrowsOnResponseManager._(PlatformManager manager) : super(manager);

  @override
  Converter<OneshotRequest<Object>, Object> get requestConverter => osToMsg;

  @override
  Converter<Object, Object> get responseConverter => msgToException;

  static Future<ThrowsOnResponseManager> create(Object entryPoint) async {
    final platformManager = await Manager.spawnWorker(entryPoint);
    return ThrowsOnResponseManager._(platformManager);
  }
}
