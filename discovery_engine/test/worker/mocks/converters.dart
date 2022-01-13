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
    show OneshotRequest, Sender, ConverterException;

class OneshotToMessageConverter
    extends Converter<OneshotRequest<Object>, List<Object>> {
  @override
  List<Object> convert(OneshotRequest<Object> input) {
    return <Object>[input.sender.platformPort, input.payload];
  }
}

class MessageToOneshotConverter
    extends Converter<List<Object>, OneshotRequest<Object>> {
  @override
  OneshotRequest<Object> convert(List<Object> input) {
    if (input.length != 2) {
      throw ArgumentError('Message to convert should be a list of 2 elements');
    }
    final sender = Sender.fromPlatformPort(input.first);
    return OneshotRequest<Object>(sender, input.last);
  }
}

class DoesNothingConverter extends Converter<Object, Object> {
  @override
  Object convert(Object input) => input;
}

class OneshotToExceptionConverter
    extends Converter<OneshotRequest<Object>, Object> {
  @override
  Object convert(OneshotRequest<Object> input) =>
      throw ConverterException('OneshotToExceptionConverter');
}

class MessageToExceptionConverter
    extends Converter<Object, OneshotRequest<Object>> {
  @override
  OneshotRequest<Object> convert(Object input) =>
      throw ConverterException('MessageToExceptionConverter');
}
