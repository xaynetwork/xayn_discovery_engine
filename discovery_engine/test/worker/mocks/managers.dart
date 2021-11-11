import 'dart:convert';

import 'package:xayn_discovery_engine/src/worker/worker.dart';

import 'converters.dart';

final osToMsg = OneshotToMessageConverter();
final msgToOs = MessageToOneshotConverter();
final defaultConverter = DoesNothingConverter();
final osToException = OneshotToExceptionConverter();
final msgToException = MessageToExceptionConverter();

class MockManager extends Manager<dynamic, dynamic> {
  MockManager._(PlatformManager manager) : super(manager);

  @override
  Converter<OneshotRequest, dynamic> get requestConverter => osToMsg;

  @override
  Converter get responseConverter => defaultConverter;

  static Future<MockManager> create(dynamic entryPoint) async {
    final platformManager = await Manager.spawnWorker(entryPoint);
    return MockManager._(platformManager);
  }
}

class ThrowsOnRequestManager extends Manager<dynamic, dynamic> {
  ThrowsOnRequestManager._(PlatformManager manager) : super(manager);

  @override
  Converter<OneshotRequest, dynamic> get requestConverter => osToException;

  @override
  Converter get responseConverter => defaultConverter;

  static Future<ThrowsOnRequestManager> create(dynamic entryPoint) async {
    final platformManager = await Manager.spawnWorker(entryPoint);
    return ThrowsOnRequestManager._(platformManager);
  }
}

class ThrowsOnResponseManager extends Manager<dynamic, dynamic> {
  ThrowsOnResponseManager._(PlatformManager manager) : super(manager);

  @override
  Converter<OneshotRequest, dynamic> get requestConverter => osToMsg;

  @override
  Converter get responseConverter => msgToException;

  static Future<ThrowsOnResponseManager> create(dynamic entryPoint) async {
    final platformManager = await Manager.spawnWorker(entryPoint);
    return ThrowsOnResponseManager._(platformManager);
  }
}
