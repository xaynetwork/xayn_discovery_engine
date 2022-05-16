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
import 'package:xayn_discovery_engine/src/api/models/active_search.dart';
import 'package:xayn_discovery_engine/src/api/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show AvailableSource, Source;
import 'package:xayn_discovery_engine/src/domain/models/trending_topic.dart';

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

/// Abstract class implemented by events like [ActiveSearchRequestSucceeded],
/// [ActiveSearchRequestFailed], [NextActiveSearchBatchRequestSucceeded],
/// [NextActiveSearchBatchRequestFailed], [RestoreActiveSearchSucceeded],
/// [RestoreActiveSearchFailed].
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
  @JsonValue(8)
  invalidEngineState,
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
    FeedFailureReason reason, {
    String? errors,
  }) = NextFeedBatchRequestFailed;

  /// Event created by the engine possibly after doing some background queries,
  /// to let the app know that there is new content available for the discovery
  /// feed. In response to that event the app may decide to show an indicator
  /// for the user that new content is ready or it might send a RestoreFeedRequested
  /// event to ask for new documents.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.nextFeedBatchAvailable() = NextFeedBatchAvailable;

  /// Event created as a success response to ExcludedSourcesListRequested event.
  /// Passes a set of [Uri] of excluded sources back to the client.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.excludedSourcesListRequestSucceeded(
    Set<Source> excludedSources,
  ) = ExcludedSourcesListRequestSucceeded;

  /// Event created as a failure response to ExcludedSourcesListRequested event.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.excludedSourcesListRequestFailed() =
      ExcludedSourcesListRequestFailed;

  @Implements<FeedEngineEvent>()
  const factory EngineEvent.trustedSourcesListRequestSucceeded(
    Set<Source> sources,
  ) = TrustedSourcesListRequestSucceeded;

  @Implements<FeedEngineEvent>()
  const factory EngineEvent.trustedSourcesListRequestFailed() =
      TrustedSourcesListRequestFailed;

  /// Event created as a success response to AvailableSourcesListRequested event.
  /// Passes a list of [AvailableSource]s back to the client.
  /// The list is sorted by decreasing match score.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.availableSourcesListRequestSucceeded(
    List<AvailableSource> availableSources,
  ) = AvailableSourcesListRequestSucceeded;

  /// Event created as a failure response to AvailableSourcesListRequested event.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.availableSourcesListRequestFailed() =
      AvailableSourcesListRequestFailed;

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

  /// Event created as a success response to various client events, like
  /// UserReactionChanged.
  @Implements<SystemEngineEvent>()
  const factory EngineEvent.clientEventSucceeded() = ClientEventSucceeded;

  /// Event created by the engine for a multitude of generic reasons, also
  /// as a "failure" event in response to various events, like
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

  /// Event created as a success response to ActiveSearchRequested event.
  /// Passes the [ActiveSearch] params and a list of [Document] entities back
  /// to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.activeSearchRequestSucceeded(
    ActiveSearch search,
    List<Document> items,
  ) = ActiveSearchRequestSucceeded;

  /// Event created as a failure response to ActiveSearchRequested event.
  /// Passes a failure reason back to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.activeSearchRequestFailed(
    SearchFailureReason reason,
  ) = ActiveSearchRequestFailed;

  /// Event created as a success response to NextActiveSearchBatchRequested event.
  /// Passes the [ActiveSearch] params and a list of [Document] entities back
  /// to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.nextActiveSearchBatchRequestSucceeded(
    ActiveSearch search,
    List<Document> items,
  ) = NextActiveSearchBatchRequestSucceeded;

  /// Event created as a failure response to NextActiveSearchBatchRequested event.
  /// Passes a failure reason back to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.nextActiveSearchBatchRequestFailed(
    SearchFailureReason reason,
  ) = NextActiveSearchBatchRequestFailed;

  /// Event created as a success response to RestoreActiveSearchRequested event.
  /// Passes the [ActiveSearch] params and a list of [Document] entities back
  /// to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.restoreActiveSearchSucceeded(
    ActiveSearch search,
    List<Document> items,
  ) = RestoreActiveSearchSucceeded;

  /// Event created as a failure response to RestoreActiveSearchRequested event.
  /// Passes a failure reason back to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.restoreActiveSearchFailed(
    SearchFailureReason reason,
  ) = RestoreActiveSearchFailed;

  /// Event created as a success response to ActiveSearchTermRequested event.
  /// Passes the current search term back to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.activeSearchTermRequestSucceeded(
    String searchTerm,
  ) = ActiveSearchTermRequestSucceeded;

  /// Event created as a failure response to ActiveSearchTermRequested event.
  /// Passes a failure reason back to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.activeSearchTermRequestFailed(
    SearchFailureReason reason,
  ) = ActiveSearchTermRequestFailed;

  /// Event created as a success response to DeepSearchRequested event.
  /// Passes a list of [Document] entities back to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.deepSearchRequestSucceeded(
    List<Document> items,
  ) = DeepSearchRequestSucceeded;

  /// Event created as a failure response to DeepSearchRequested event.
  /// Passes a failure reason back to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.deepSearchRequestFailed(
    SearchFailureReason reason,
  ) = DeepSearchRequestFailed;

  /// Event created as a success response to TrendingTopicsRequested event.
  /// Passes a list of [TrendingTopic] entities back to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.trendingTopicsRequestSucceeded(
    List<TrendingTopic> topics,
  ) = TrendingTopicsRequestSucceeded;

  /// Event created as a failure response to TrendingTopicsRequested event.
  /// Passes a failure reason back to the client.
  @Implements<SearchEngineEvent>()
  const factory EngineEvent.trendingTopicsRequestFailed(
    SearchFailureReason reason,
  ) = TrendingTopicsRequestFailed;

  /// Converts json Map to [EngineEvent].
  factory EngineEvent.fromJson(Map<String, Object?> json) =>
      _$EngineEventFromJson(json);
}
