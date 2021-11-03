import 'dart:async' show Stream;

abstract class PlatformActor {
  /// Stream of incoming messages
  Stream get messages;

  /// Method for sending messages to the other [PlatformActor]
  void send(dynamic message, [List<Object>? transfer]);

  /// Method for performing platform specific cleanup. It's called
  /// by the wrapper class that makes use of [PlatformActor].
  void dispose();
}

/// Base class for PlatformManager actor
abstract class PlatformManager extends PlatformActor {}

/// Base class for PlatformWorker actor
abstract class PlatformWorker extends PlatformActor {}
