/// Base for all event classes
abstract class Event {
  const Event();
}

/// For events sent from the app to the engine
abstract class ClientEvent extends Event {
  const ClientEvent();
}

/// For events sent from the engine to the app
abstract class EngineEvent extends Event {
  const EngineEvent();
}
