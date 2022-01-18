import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart' show asyncCore;

void main() {
  test('calling async ffi functions works', () async {
    final x = await asyncCore.add(10, 22);
    expect(x, equals(32));
  });
}
