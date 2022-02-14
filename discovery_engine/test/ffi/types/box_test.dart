import 'dart:ffi';

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;

void main() {
  const address = 4;
  final dangling = Pointer<Uint32>.fromAddress(address);

  test('ref/mut point to the right address', () {
    final box = Boxed(dangling, (_) {
      throw AssertionError('free is not supposed to be called here');
    });
    expect(box.ref, equals(dangling));
    expect(box.mut, equals(dangling));
  });
  
  test('move moves the ownership', () {
    final box = Boxed(dangling, (_) {
      throw AssertionError('free is not supposed to be called here');
    });
    expect(box.moved, isFalse);
    expect(box.ref, equals(dangling));
    expect(box.mut, equals(dangling));
    expect(box.move(), equals(dangling));
    expect(box.moved, isTrue);
    expect(() {
      box.ref;
    }, throwsStateError,);
    expect(() {
      box.mut;
    }, throwsStateError,);
  });

  test('free calls free and moves ownership', () {
    var freed = false;
    final box = Boxed(dangling, (ptr) {
      expect(ptr, equals(dangling));
      freed = true;
    });
    expect(box.moved, isFalse);
    expect(box.ref, equals(dangling));
    box.free();
    expect(freed, isTrue);
    expect(box.moved, isTrue);
  });
}
