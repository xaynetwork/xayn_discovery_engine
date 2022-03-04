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

import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/api/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart';

part 'engine_events.freezed.dart';
part 'engine_events.g.dart';

/// Abstract class implemented by events like [RestoreFeedSucceeded],
/// [RestoreFeedFailed], [NextFeedBatchRequestSucceeded],
/// [NextFeedBatchRequestFailed] or [NextFeedBatchAvailable].
///
/// Used to group discovery feed related events.
abstract class FeedEngineEvent implements EngineEvent {}

/// Abstract class implemented by events like [ClientEventSucceeded] or
/// [EngineExceptionRaised].
///
/// Used to group generic system events.
abstract class SystemEngineEvent implements EngineEvent {}

/// Abstract class implemented by events used to communicate status of
/// AI assets fetching process.
abstract class AssetsStatusEngineEvent implements EngineEvent {}

/// Abstract class implemented by events like [DocumentsUpdated].
///
/// Used to group events related to [Document] changes.
abstract class DocumentEngineEvent implements EngineEvent {}

/// Abstract class implemented by events like [SearchRequestSucceeded],
/// [SearchRequestFailed], [NextSearchBatchRequestSucceeded],
/// [NextSearchBatchRequestFailed], [RestoreSearchSucceeded],
/// [RestoreSearchFailed].
///
/// Used to group active search related events.
abstract class SearchEngineEvent implements EngineEvent {}

enum FeedFailureReason {
  @JsonValue(0)
  notAuthorised,
  @JsonValue(1)
  noNewsForMarket,
  @JsonValue(2)
  stacksOpsError,
}

enum SearchFailureReason {
  @JsonValue(0)
  noActiveSearch,
  @JsonValue(1)
  noResultsAvailable,
}

enum EngineExceptionReason {
  @JsonValue(0)
  genericError,
  @JsonValue(1)
  engineNotReady,
  @JsonValue(2)
  wrongEventRequested,
  @JsonValue(3)
  wrongEventInResponse,
  @JsonValue(4)
  converterException,
  @JsonValue(5)
  responseTimeout,
  @JsonValue(6)
  engineDisposed,
  @JsonValue(7)
  failedToGetAssets,
  // other possible errors will be added below
}

@freezed
class EngineEvent with _$EngineEvent {
  /// Event created as a success response to RestoreFeedRequested event.
  /// Passes a list of [Document] entities back to the client.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.restoreFeedSucceeded(List<Document> items) =
      RestoreFeedSucceeded;

  /// Event created as a failure response to RestoreFeedRequested event.
  ///
  /// Passes back a failure reason that the client can use to determine
  /// how to react, e.g. display user friendly messages, repeat request, etc.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.restoreFeedFailed(FeedFailureReason reason) =
      RestoreFeedFailed;

  /// Event created as a success response to NextFeedBatchRequested event.
  /// Passes a list of [Document] entities back to the client.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.nextFeedBatchRequestSucceeded(
    List<Document> items,
  ) = NextFeedBatchRequestSucceeded;

  /// Event created as a failure response to NextFeedBatchRequested event.
  ///
  /// Passes back a failure reason that the client can use to determine
  /// how to react, e.g. display user friendly messages, repeat request, etc.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.nextFeedBatchRequestFailed(
    FeedFailureReason reason,
    String? errors,
  ) = NextFeedBatchRequestFailed;

  /// Event created by the engine possibly after doing some background queries,
  /// to let the app know that there is new content available for the discovery
  /// feed. In response to that event the app may decide to show an indicator
  /// for the user that new content is ready or it might send a RestoreFeedRequested
  /// event to ask for new documents.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.nextFeedBatchAvailable() = NextFeedBatchAvailable;

  /// Event created when fetching of AI assets has started.
  @Implements<AssetsStatusEngineEvent>()
  const factory EngineEvent.fetchingAssetsStarted() = FetchingAssetsStarted;

  /// Event created when fetching of AI assets has progressed.
  @Implements<AssetsStatusEngineEvent>()
  const factory EngineEvent.fetchingAssetsProgressed(double percentage) =
      FetchingAssetsProgressed;

  /// Event created when fetching of AI assets has finished.
  @Implements<AssetsStatusEngineEvent>()
  const factory EngineEvent.fetchingAssetsFinished() = FetchingAssetsFinished;

  /// Event created to inform the client that a particular "fire and forget"
  /// event, like UserReactionChanged, was successfuly processed
  /// by the engine.
  @Implements<SystemEngineEvent>()
  const factory EngineEvent.clientEventSucceeded() = ClientEventSucceeded;

  /// Event created by the engine for a multitude of generic reasons, also
  /// as a "failure" event in response to "fire and forget" events, like
  /// UserReactionChanged.
  @Implements<SystemEngineEvent>()
  const factory EngineEvent.engineExceptionRaised(
    EngineExceptionReason reason, {
    String? message,
    String? stackTrace,
  }) = EngineExceptionRaised;

  /// Event created as a success response to some client events which are
  /// updating a [Document] (currently only "UserReactionChanged").
  /// Passes back to the client a list of changed [Document] entities.
  @Implements<DocumentEngineEvent>()
  const factory EngineEvent.documentsUpdated(List<Document> items) =
      DocumentsUpdated;

  /// Event created as a success response to SearchRequested event.
  /// Passes the [ActiveSearch] params and a list of [Document] entities back
  /// to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.searchRequestSucceeded(
    ActiveSearch search,
    List<Document> items,
  ) = SearchRequestSucceeded;

  /// Event created as a failure response to SearchRequested event.
  /// Passes a failure reason back to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.searchRequestFailed(
    SearchFailureReason reason,
  ) = SearchRequestFailed;

  /// Event created as a success response to NextSearchBatchRequested event.
  /// Passes the [ActiveSearch] params and a list of [Document] entities back
  /// to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.nextSearchBatchRequestSucceeded(
    ActiveSearch search,
    List<Document> items,
  ) = NextSearchBatchRequestSucceeded;

  /// Event created as a failure response to NextSearchBatchRequested event.
  /// Passes a failure reason back to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.nextSearchBatchRequestFailed(
    SearchFailureReason reason,
  ) = NextSearchBatchRequestFailed;

  /// Event created as a success response to RestoreSearchRequested event.
  /// Passes the [ActiveSearch] params and a list of [Document] entities back
  /// to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.restoreSearchSucceeded(
    ActiveSearch search,
    List<Document> items,
  ) = RestoreSearchSucceeded;

  /// Event created as a failure response to RestoreSearchRequested event.
  /// Passes a failure reason back to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.restoreSearchFailed(
    SearchFailureReason reason,
  ) = RestoreSearchFailed;

  /// Converts json Map to [EngineEvent].
  factory EngineEvent.fromJson(Map<String, Object?> json) =>
      _$EngineEventFromJson(json);
}
