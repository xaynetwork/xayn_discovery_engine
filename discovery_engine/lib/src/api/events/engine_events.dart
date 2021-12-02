import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/api/models/document.dart';

part 'engine_events.freezed.dart';
part 'engine_events.g.dart';

abstract class FeedEngineEvent {}

abstract class SystemEngineEvent {}

enum FeedFailureReason {
  @JsonValue(0)
  notAuthorised,
  @JsonValue(1)
  noNewsForMarket,
}

enum EngineExceptionReason {
  @JsonValue(0)
  genericError,
  @JsonValue(1)
  noInitReceived,
  @JsonValue(1)
  wrongEventInResponse,
  // other possible errors will be added below
}

@freezed
class EngineEvent with _$EngineEvent {
  /// Event created as a successful response to FeedRequested event.
  /// Passes back a list of [Document] entities back to the client.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.feedRequestSucceeded(List<Document> items) =
      FeedRequestSucceeded;

  /// Event created as a failure response to FeedRequested event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.feedRequestFailed(FeedFailureReason reason) =
      FeedRequestFailed;

  /// Event created as a successful response to NextFeedBatchRequested event.
  /// Passes back a list of [Document] objects back to the client.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.nextFeedBatchRequestSucceeded(
    List<Document> items,
  ) = NextFeedBatchRequestSucceeded;

  /// Event created as a failure response to NextFeedBatchRequested event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.nextFeedBatchRequestFailed(
    FeedFailureReason reason,
  ) = NextFeedBatchRequestFailed;

  /// Event created by the engine, possibly after doing some background queries
  /// to let the app know that there is new content available for the discovery
  /// feed. In response to that event the app may decide to show an indicator
  /// for the user that new content is ready or it might send FeedRequested
  /// event to ask for new documents.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.nextFeedBatchAvailable() = NextFeedBatchAvailable;

  /// Event created to inform the client that a particular "fire and forget"
  /// event, like ie. DocumentFeedbackChanged, was successfuly processed
  /// by the engine.
  @Implements<SystemEngineEvent>()
  const factory EngineEvent.clientEventSucceeded() = ClientEventSucceeded;

  /// Event created by the engine for multitude of generic reasons, also
  /// as a "failure" event in response to "fire and forget" events, like
  /// ie. DocumentFeedbackChanged.
  @Implements<SystemEngineEvent>()
  const factory EngineEvent.engineExceptionRaised(
    EngineExceptionReason reason,
  ) = EngineExceptionRaised;

  /// Converts json Map to [EngineEvent].
  factory EngineEvent.fromJson(Map<String, dynamic> json) =>
      _$EngineEventFromJson(json);
}
