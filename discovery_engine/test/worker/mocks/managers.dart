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
