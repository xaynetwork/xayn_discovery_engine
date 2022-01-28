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

part 'engine_events.freezed.dart';
part 'engine_events.g.dart';

/// Abstract class implemented by events like [FeedRequestSucceeded],
/// [FeedRequestFailed], [NextFeedBatchRequestSucceeded],
/// [NextFeedBatchRequestFailed] or [NextFeedBatchAvailable].
///
/// Used to group discovery feed related events.
abstract class FeedEngineEvent {}

/// Abstract class implemented by events like [ClientEventSucceeded] or
/// [EngineExceptionRaised].
///
/// Used to group generic system events.
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
  /// Event created as a successful response to FeedRequested event.
  /// Passes back a list of [Document] entities back to the client.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.feedRequestSucceeded(List<Document> items) =
      FeedRequestSucceeded;

  /// Event created as a failure response to FeedRequested event.
  ///
  /// Passes back a failure reason that the client can use to determine
  /// how to react, e.g. display user friendly messages, repeat request, etc.
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
  /// Passes back a failure reason that the client can use to determine
  /// how to react, e.g. display user friendly messages, repeat request, etc.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.nextFeedBatchRequestFailed(
    FeedFailureReason reason,
  ) = NextFeedBatchRequestFailed;

  /// Event created by the engine possibly after doing some background queries,
  /// to let the app know that there is new content available for the discovery
  /// feed. In response to that event the app may decide to show an indicator
  /// for the user that new content is ready or it might send a FeedRequested
  /// event to ask for new documents.
  @Implements<FeedEngineEvent>()
  const factory EngineEvent.nextFeedBatchAvailable() = NextFeedBatchAvailable;

  /// Event created when fetching of AI assets has started.
  @Implements<SystemEngineEvent>()
  const factory EngineEvent.fetchingAssetsStarted() = FetchingAssetsStarted;

  /// Event created when fetching of AI assets has progressed.
  @Implements<SystemEngineEvent>()
  const factory EngineEvent.fetchingAssetsProgressed(double percentage) =
      FetchingAssetsProgressed;

  /// Event created when fetching of AI assets has finished.
  @Implements<SystemEngineEvent>()
  const factory EngineEvent.fetchingAssetsFinished() = FetchingAssetsFinished;

  /// Event created to inform the client that a particular "fire and forget"
  /// event, like DocumentFeedbackChanged, was successfuly processed
  /// by the engine.
  @Implements<SystemEngineEvent>()
  const factory EngineEvent.clientEventSucceeded() = ClientEventSucceeded;

  /// Event created by the engine for a multitude of generic reasons, also
  /// as a "failure" event in response to "fire and forget" events, like
  /// DocumentFeedbackChanged.
  @Implements<SystemEngineEvent>()
  const factory EngineEvent.engineExceptionRaised(
    EngineExceptionReason reason,
  ) = EngineExceptionRaised;

  /// Converts json Map to [EngineEvent].
  factory EngineEvent.fromJson(Map<String, Object?> json) =>
      _$EngineEventFromJson(json);
}
