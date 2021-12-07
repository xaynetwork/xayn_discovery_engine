/// Thrown when a Manager failed to initialize.
class EngineInitException implements Exception {
  /// Message (or string representation of the exception).
  final String message;

  /// Original exception that was the cause
  final Object exception;

  EngineInitException(this.message, this.exception);

  @override
  String toString() =>
      'EngineInitException: message: $message; exception: $exception';
}

/// Thrown when a Worker cannot be created.
class WorkerSpawnException implements Exception {
  /// Message (or string representation of the exception).
  final String message;

  /// String representation of the stack trace associated to the original exception.
  final StackTrace? stackTrace;

  WorkerSpawnException(this.message, {StackTrace? stackTrace})
      : stackTrace = stackTrace ?? StackTrace.current;

  @override
  String toString() => 'WorkerSpawnException: $message\n$stackTrace';
}

/// Thrown when a Converter cannot convert a message.
class ConverterException implements Exception {
  /// Message (or string representation of the exception).
  final String message;

  // The original payload that caused conversion failure
  final Object? payload;

  // The original exception thrown during failed conversion
  final Object? source;

  ConverterException(this.message, {this.payload, this.source});

  @override
  String toString() => '''ConverterException: $message;
    Payload: ${payload ?? 'none'};
    Source: ${source ?? 'none'}
    ''';
}

/// Thrown when the Response is empty (resolved to `null`).
class ResponseEmptyException implements Exception {
  /// Message (or string representation of the exception).
  final String message;

  ResponseEmptyException(this.message);

  @override
  String toString() => 'ResponseEmptyException: $message';
}

/// Thrown when the Response takes too long to finish.
class ResponseTimeoutException implements Exception {
  /// Message (or string representation of the exception).
  final String message;

  ResponseTimeoutException(this.message);

  @override
  String toString() => 'ResponseTimeoutException: $message';
}
