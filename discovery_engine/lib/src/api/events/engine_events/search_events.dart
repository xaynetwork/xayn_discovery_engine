import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/api/events/engine_groups.dart'
    show EngineEvent;
import 'package:xayn_discovery_engine/src/api/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';

part 'search_events.freezed.dart';
part 'search_events.g.dart';

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
class SearchEngineEvent with _$SearchEngineEvent implements EngineEvent {
  /// Event created as a successful response to SearchRequested event.
  ///
  /// Passes back list of [Document] objects together with [SearchId] to indicate
  /// which "search" these objects belong to.
  const factory SearchEngineEvent.searchRequestSucceeded(
    SearchId searchId,
    List<Document> items,
  ) = SearchRequestSucceeded;

  /// Event created as a failure response to SearchRequested event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  const factory SearchEngineEvent.searchRequestFailed(
    SearchFailureReason reason,
  ) = SearchRequestFailed;

  /// Event created as a successful response to NextSearchBatchRequested event.
  ///
  /// Passes back list of [Document] objects for the next page/batch.
  const factory SearchEngineEvent.nextSearchBatchRequestSucceeded(
    List<Document> items,
  ) = NextSearchBatchRequestSucceeded;

  /// Event created as a failure response to NextSearchBatchRequested event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  const factory SearchEngineEvent.nextSearchBatchRequestFailed(
    SearchFailureReason reason,
  ) = NextSearchBatchRequestFailed;

  /// Event created as a successful response to SearchRestoreRequested event.
  ///
  /// Passes back list of all [Document] objects related to previously performed
  /// search that the client requested to restore.
  const factory SearchEngineEvent.searchRestoreRequestSucceeded(
    List<Document> items,
  ) = SearchRestoreRequestSucceeded;

  /// Event created as a failure response to SearchRestoreRequested event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  const factory SearchEngineEvent.searchRestoreRequestFailed(
    SearchRestoreFailureReason reason,
  ) = SearchRestoreRequestFailed;

  /// Event created as a successful response to ActiveSearchesRequested event.
  ///
  /// Passes back list of all [SearchId] objects that the client can then request
  /// to restore.
  const factory SearchEngineEvent.activeSearchesRequestSucceeded(
    Set<SearchId> searchIds,
  ) = ActiveSearchesRequestSucceeded;

  /// Event created as a failure response to ActiveSearchesRequested event.
  ///
  /// Passes back a failure reason, that the client can use to determine
  /// how to react, ie. display user friendly messages, repeat request, etc.
  const factory SearchEngineEvent.activeSearchesRequestFailed(
    ActiveSearchesFailure reason,
  ) = ActiveSearchesRequestFailed;

  /// Converts json Map to [SearchEngineEvent].
  factory SearchEngineEvent.fromJson(Map<String, dynamic> json) =>
      _$SearchEngineEventFromJson(json);
}
