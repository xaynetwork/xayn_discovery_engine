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
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart';

part 'client_events.freezed.dart';
part 'client_events.g.dart';

/// Abstract class implemented by events like [DocumentFeedbackChanged].
///
/// Used to group events related to [Document] changes.
abstract class DocumentClientEvent implements ClientEvent {}

/// Abstract class implemented by events like [FeedRequested],
/// [NextFeedBatchRequested] or [FeedDocumentsClosed].
///
/// Used to group discovery feed related events.
abstract class FeedClientEvent implements ClientEvent {}

/// Abstract class implemented by events like [Init], [ResetEngine] or
/// [ConfigurationChanged].
///
/// Used to group generic system events.
abstract class SystemClientEvent implements ClientEvent {}

@freezed
class ClientEvent with _$ClientEvent {
  /// Event created upon every app startup, with some data needed
  /// for the engine to work, like personalisation and feed market
  /// (for performing background queries).
  @Implements<SystemClientEvent>()
  const factory ClientEvent.init(
    Configuration configuration,
  ) = Init;

  /// Event created when the app decides to reset the AI (start fresh).
  @Implements<SystemClientEvent>()
  const factory ClientEvent.resetEngine() = ResetEngine;

  /// Event created when the user changes market or count (nb of items per page)
  /// for the feed ie. in global settings.
  @Implements<SystemClientEvent>()
  @Assert('feedMarkets == null || feedMarkets.length>0')
  const factory ClientEvent.configurationChanged({
    FeedMarkets? feedMarkets,
    int? maxItemsPerFeedBatch,
  }) = ConfigurationChanged;

  /// Event created when opening up discovery screen (upon initial start
  /// of the app or when we are returning to the previously displayed
  /// discovery feed). When restoring the previous feed it returns all the documents
  /// that were still accessible to the user, namely those that weren't closed in
  /// the [FeedDocumentsClosed] event.
  @Implements<FeedClientEvent>()
  const factory ClientEvent.feedRequested() = FeedRequested;

  /// Event created when the app wants to request new content
  /// for the discovery feed:
  ///  - when reaching the end of the current list of items
  ///  - in response to `NextFeedBatchAvailable` event, or after deliberate user action
  /// like pressing the button to fetch new items
  ///  - on some time trigger
  ///  - as a follow up when changing the configuration
  @Implements<FeedClientEvent>()
  const factory ClientEvent.nextFeedBatchRequested() = NextFeedBatchRequested;

  /// Event created when the client makes [Document]s in the feed not accessible
  /// to the user anymore. The engine registers those documents as immutable,
  /// so they can't be changed anymore by the client.
  @Implements<FeedClientEvent>()
  const factory ClientEvent.feedDocumentsClosed(
    Set<DocumentId> documentIds,
  ) = FeedDocumentsClosed;

  /// Event created when a [Document] has been viewed in a certain mode for
  /// the given amount of time in seconds.
  @Implements<DocumentClientEvent>()
  const factory ClientEvent.documentTimeLogged(
    DocumentId documentId,
    DocumentViewMode mode,
    int seconds,
  ) = DocumentTimeLogged;

  /// Event created when the user swipes the [Document] card or clicks a button
  /// to indicate that the document is `positive`, `negative` or `neutral`.
  @Implements<DocumentClientEvent>()
  const factory ClientEvent.documentFeedbackChanged(
    DocumentId documentId,
    DocumentFeedback feedback,
  ) = DocumentFeedbackChanged;

  /// Converts json Map to [ClientEvent].
  factory ClientEvent.fromJson(Map<String, Object?> json) =>
      _$ClientEventFromJson(json);
}
