import 'dart:html' show querySelector;
import 'package:example/example.dart' show runExample;

void main() {
  querySelector('#output')?.text = 'Your Dart app is running.';
  runExample();
}
