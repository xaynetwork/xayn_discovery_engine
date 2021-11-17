import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/api/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';

part 'engine_events.freezed.dart';
part 'engine_events.g.dart';

enum EngineExceptionReason {
  @JsonValue(0)
  noInitReceived,
  // other possible errors will be added below
}

enum FeedFailureReason {
  @JsonValue(0)
  notAuthorised,
  @JsonValue(1)
  noNewsForMarket,
}

enum SearchFailureReason {
  @JsonValue(0)
  notAuthorised,
  @JsonValue(1)
  notFound,
}

enum SearchRestoreFailureReason {
  @JsonValue(0)
  notFound,
  @JsonValue(1)
  searchClosed,
}

enum ActiveSearchesFailure {
  @JsonValue(0)
  notFound,
  // etc.
}

@freezed
class EngineEvent with _$EngineEvent {
  /// Event created as a successful response to FeedRequested event.
  /// Passes back a list of [Document] entities back to the client.
  const factory EngineEvent.feedRequestSucceeded(List<Document> items) =
      FeedRequestSucceeded;

  /// Event created as a failure response to FeedRequested event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  const factory EngineEvent.feedRequestFailed(FeedFailureReason reason) =
      FeedRequestFailed;

  /// Event created as a successful response to NewFeedRequested event.
  /// Passes back a list of [Document] objects back to the client.
  const factory EngineEvent.newFeedRequestSucceeded(List<Document> items) =
      NewFeedRequestSucceeded;

  /// Event created as a failure response to NewFeedRequested event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  const factory EngineEvent.newFeedRequestFailed(FeedFailureReason reason) =
      NewFeedRequestFailed;

  /// Event created by the engine, possibly after doing some background queries
  /// to let the app know that there is new content available for the discovery
  /// feed. In response to that event the app may decide to show an indicator
  /// for the user that new content is ready or it might send FeedRequested
  /// event to ask for new documents.
  const factory EngineEvent.newFeedAvailable() = NewFeedAvailable;

  /// Event created as a successful response to SearchRequested event.
  ///
  /// Passes back list of [Document] objects together with [SearchId] to indicate
  /// which "search" these objects belong to.
  const factory EngineEvent.searchRequestSucceeded(
    SearchId searchId,
    List<Document> items,
  ) = SearchRequestSucceeded;

  /// Event created as a failure response to [SearchRequested] event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  const factory EngineEvent.searchRequestFailed(SearchFailureReason reason) =
      SearchRequestFailed;

  /// Event created as a successful response to NextSearchBatchRequested event.
  ///
  /// Passes back list of [Document] objects for the next page/batch.
  const factory EngineEvent.nextSearchBatchRequestSucceeded(
    List<Document> items,
  ) = NextSearchBatchRequestSucceeded;

  /// Event created as a failure response to NextSearchBatchRequested event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  const factory EngineEvent.nextSearchBatchRequestFailed(
    SearchFailureReason reason,
  ) = NextSearchBatchRequestFailed;

  /// Event created as a successful response to SearchRestoreRequested event.
  ///
  /// Passes back list of all [Document] objects related to previously performed
  /// search that the client requested to restore.
  const factory EngineEvent.searchRestoreRequestSucceeded(
    List<Document> items,
  ) = SearchRestoreRequestSucceeded;

  /// Event created as a failure response to SearchRestoreRequested event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  const factory EngineEvent.searchRestoreRequestFailed(
    SearchRestoreFailureReason reason,
  ) = SearchRestoreRequestFailed;

  /// Event created as a successful response to ActiveSearchesRequested event.
  ///
  /// Passes back list of all [SearchId] objects that the client can then request
  /// to restore.
  const factory EngineEvent.activeSearchesRequestSucceeded(
    Set<SearchId> searchIds,
  ) = ActiveSearchesRequestSucceeded;

  /// Event created as a failure response to ActiveSearchesRequested event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  const factory EngineEvent.activeSearchesRequestFailed(
    ActiveSearchesFailure reason,
  ) = ActiveSearchesRequestFailed;

  /// Event created to inform the client that a particular "fire and forget"
  /// event, like ie. DocumentFeedbackChanged, was successfuly processed
  /// by the engine.
  const factory EngineEvent.clientEventSucceeded() = ClientEventSucceeded;

  /// Event created by the engine for multitude of generic reasons, also
  /// as a "failure" event in response to "fire and forget" events, like
  /// ie. DocumentFeedbackChanged.
  const factory EngineEvent.engineExceptionRaised(
    EngineExceptionReason reason,
  ) = EngineExceptionRaised;

  /// Event created as a response to UrlOpened event which
  /// contains [DocumentId] to be used with other "document" events,
  /// like DocumentClosed, DocumentFeedbackChanged, etc.
  const factory EngineEvent.documentFromUrlCreated(DocumentId documentId) =
      DocumentFromUrlCreated;

  /// Converts json Map to [EngineEvent].
  factory EngineEvent.fromJson(Map<String, dynamic> json) =>
      _$EngineEventFromJson(json);
}
