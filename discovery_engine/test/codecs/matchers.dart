import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show ConverterException;

/// A type matcher for [ConverterException].
final isConverterException = isA<ConverterException>();

/// A matcher for [ConverterException].
final throwsConverterException = throwsA(isConverterException);
