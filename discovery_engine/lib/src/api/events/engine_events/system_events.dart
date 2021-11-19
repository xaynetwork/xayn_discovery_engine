import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/api/events/engine_groups.dart'
    show EngineEvent;

part 'system_events.freezed.dart';
part 'system_events.g.dart';

enum EngineExceptionReason {
  @JsonValue(0)
  noInitReceived,
  // other possible errors will be added below
}

@freezed
class SystemEngineEvent with _$SystemEngineEvent implements EngineEvent {
  /// Event created to inform the client that a particular "fire and forget"
  /// event, like ie. DocumentFeedbackChanged, was successfuly processed
  /// by the engine.
  const factory SystemEngineEvent.clientEventSucceeded() = ClientEventSucceeded;

  /// Event created by the engine for multitude of generic reasons, also
  /// as a "failure" event in response to "fire and forget" events, like
  /// ie. DocumentFeedbackChanged.
  const factory SystemEngineEvent.engineExceptionRaised(
    EngineExceptionReason reason,
  ) = EngineExceptionRaised;

  /// Converts json Map to [SystemEngineEvent].
  factory SystemEngineEvent.fromJson(Map<String, dynamic> json) =>
      _$SystemEngineEventFromJson(json);
}
