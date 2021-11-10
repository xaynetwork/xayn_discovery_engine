/// Thrown when a Worker cannot be created.
class WorkerSpawnException implements Exception {
  /// Message (or string representation of the exception).
  final String message;

  /// String representation of the stack trace associated to the original exception.
  final String? stackTrace;

  WorkerSpawnException(this.message, {String? stackTrace})
      : stackTrace = stackTrace ?? StackTrace.current.toString();

  @override
  String toString() => 'WorkerSpawnException: $message\n$stackTrace';
}

/// Thrown when a Converter cannot convert a message.
class ConverterException implements Exception {
  /// Message (or string representation of the exception).
  final String message;

  ConverterException(this.message);

  @override
  String toString() => 'ConverterException: $message';
}

/// Thrown when the Response takes too long to finish.
class ResponseTimeoutException implements Exception {
  /// Message (or string representation of the exception).
  final String message;

  ResponseTimeoutException(this.message);

  @override
  String toString() => 'ResponseTimeoutException: $message';
}
