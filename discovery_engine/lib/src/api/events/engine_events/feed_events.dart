import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/api/events/engine_groups.dart'
    show EngineEvent;
import 'package:xayn_discovery_engine/src/api/models/document.dart';

part 'feed_events.freezed.dart';
part 'feed_events.g.dart';

enum FeedFailureReason {
  @JsonValue(0)
  notAuthorised,
  @JsonValue(1)
  noNewsForMarket,
}

@freezed
class FeedEngineEvent with _$FeedEngineEvent implements EngineEvent {
  /// Event created as a successful response to FeedRequested event.
  /// Passes back a list of [Document] entities back to the client.
  const factory FeedEngineEvent.feedRequestSucceeded(List<Document> items) =
      FeedRequestSucceeded;

  /// Event created as a failure response to FeedRequested event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  const factory FeedEngineEvent.feedRequestFailed(FeedFailureReason reason) =
      FeedRequestFailed;

  /// Event created as a successful response to NextFeedBatchRequested event.
  /// Passes back a list of [Document] objects back to the client.
  const factory FeedEngineEvent.nextFeedBatchRequestSucceeded(
    List<Document> items,
  ) = NextFeedBatchRequestSucceeded;

  /// Event created as a failure response to NextFeedBatchRequested event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  const factory FeedEngineEvent.nextFeedBatchRequestFailed(
    FeedFailureReason reason,
  ) = NextFeedBatchRequestFailed;

  /// Event created by the engine, possibly after doing some background queries
  /// to let the app know that there is new content available for the discovery
  /// feed. In response to that event the app may decide to show an indicator
  /// for the user that new content is ready or it might send FeedRequested
  /// event to ask for new documents.
  const factory FeedEngineEvent.nextFeedBatchAvailable() =
      NextFeedBatchAvailable;

  /// Converts json Map to [FeedEngineEvent].
  factory FeedEngineEvent.fromJson(Map<String, dynamic> json) =>
      _$FeedEngineEventFromJson(json);
}
