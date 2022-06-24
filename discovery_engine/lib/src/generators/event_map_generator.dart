// Copyright 2022 Xayn AG
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

import 'package:analyzer/dart/element/element.dart';
import 'package:build/build.dart';
import 'package:source_gen/source_gen.dart';
import 'package:xayn_discovery_engine/src/generators/annotations.dart';
import 'package:xayn_discovery_engine/src/generators/visitors/class_name_visitor.dart';
import 'package:xayn_discovery_engine/src/generators/visitors/const_member_name_visitor.dart';

class EngineEventMapGenerator extends GeneratorForAnnotation<GenerateEventMap> {
  @override
  String generateForAnnotatedElement(
    Element element,
    ConstantReader annotation,
    BuildStep buildStep,
  ) {
    final classNameVisitor = ClassNameVisitor();
    element.visitChildren(classNameVisitor);

    final constNameVisitor = ConstMemberNameVisitor();
    element.visitChildren(constNameVisitor);

    final buffer = StringBuffer();
    buffer.writeln('extension MapEvent on EngineEvent {');
    buffer.writeln('${classNameVisitor.className} mapEvent({');

    for (final el in constNameVisitor.constMemberNames) {
      buffer.writeln('bool? $el,');
    }

    // closes the parameter list and opens the `map` function call
    buffer.writeln('}) => map(');

    for (final el in constNameVisitor.constMemberNames) {
      buffer.writeln('$el: _maybePassThrough($el),');
    }

    buffer.writeln(');'); // closes the `map` function call

    buffer.writeln(
      '''
      EngineEvent Function(EngineEvent) _maybePassThrough(bool? condition) {
        return condition ?? false ? _passThrough : _orElse;
      }

      // just pass through the original event
      ${classNameVisitor.className} _passThrough(EngineEvent event) => event;

      // in case of a wrong event in response create an EngineExceptionRaised
      EngineEvent _orElse(EngineEvent event) =>
          const EngineEvent.engineExceptionRaised(
            EngineExceptionReason.wrongEventInResponse,
          );
    ''',
    );

    // closes the extension
    buffer.writeln('}');

    return buffer.toString();
  }
}
