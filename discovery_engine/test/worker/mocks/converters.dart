import 'dart:convert' show Converter;

import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show OneshotRequest, Sender, ConverterException;

class OneshotToMessageConverter extends Converter<OneshotRequest, dynamic> {
  @override
  dynamic convert(OneshotRequest input) {
    return <dynamic>[input.sender.platformPort, input.payload];
  }
}

class MessageToOneshotConverter extends Converter<dynamic, OneshotRequest> {
  @override
  OneshotRequest convert(dynamic input) {
    final list = input as List;
    final sender = Sender.fromPlatformPort(list.first as Object);
    return OneshotRequest<dynamic>(sender, list.last);
  }
}

class DoesNothingConverter extends Converter<dynamic, dynamic> {
  @override
  dynamic convert(dynamic input) => input;
}

class OneshotToExceptionConverter extends Converter<OneshotRequest, dynamic> {
  @override
  dynamic convert(OneshotRequest input) =>
      throw ConverterException('OneshotToExceptionConverter');
}

class MessageToExceptionConverter extends Converter<dynamic, OneshotRequest> {
  @override
  OneshotRequest convert(dynamic input) =>
      throw ConverterException('MessageToExceptionConverter');
}
