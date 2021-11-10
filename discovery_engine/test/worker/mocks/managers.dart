import 'dart:convert';

import 'package:xayn_discovery_engine/src/worker/worker.dart';

import 'converters.dart';

final osToMsg = OneshotToMessageConverter();
final msgToOs = MessageToOneshotConverter();
final defaultConverter = DoesNothingConverter();
final osToException = OneshotToExceptionConverter();
final msgToException = MessageToExceptionConverter();

class MockManager extends Manager<dynamic, dynamic> {
  MockManager(dynamic entryPoint) : super(entryPoint);

  @override
  Converter<OneshotRequest, dynamic> get requestConverter => osToMsg;

  @override
  Converter get responseConverter => defaultConverter;
}

class ThrowsOnRequestManager extends MockManager {
  ThrowsOnRequestManager(dynamic entryPoint) : super(entryPoint);

  @override
  Converter<OneshotRequest, dynamic> get requestConverter => osToException;
}

class ThrowsOnResponseManager extends MockManager {
  ThrowsOnResponseManager(dynamic entryPoint) : super(entryPoint);

  @override
  Converter get responseConverter => msgToException;
}
