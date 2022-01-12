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

/// Thrown when the Manager tries to send something after being disposed.
class ManagerDisposedException implements Exception {
  /// Message (or string representation of the exception).
  final String message;

  ManagerDisposedException(this.message);

  @override
  String toString() => 'ManagerDisposedException: $message';
}

/// Thrown when the Response takes too long to finish.
class ResponseTimeoutException implements Exception {
  /// Message (or string representation of the exception).
  final String message;

  ResponseTimeoutException(this.message);

  @override
  String toString() => 'ResponseTimeoutException: $message';
}
