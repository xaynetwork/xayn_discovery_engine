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
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    show SearchBy;
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart';
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show Source;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart';

part 'client_events.freezed.dart';
part 'client_events.g.dart';

/// Abstract class implemented by events like [UserReactionChanged].
///
/// Used to group events related to [Document] changes.
abstract class DocumentClientEvent implements ClientEvent {}

/// Abstract class implemented by events like [RestoreFeedRequested],
/// [NextFeedBatchRequested] or [FeedDocumentsClosed].
///
/// Used to group discovery feed related events.
abstract class FeedClientEvent implements ClientEvent {}

/// Abstract class implemented by events like [Init] or
/// [ConfigurationChanged].
///
/// Used to group generic system events.
abstract class SystemClientEvent implements ClientEvent {}

/// Abstract class implemented by events like [ActiveSearchRequested],
/// [RestoreActiveSearchRequested] or [ActiveSearchClosed].
///
/// Used to group active search related events.
abstract class SearchClientEvent implements ClientEvent {}

@freezed
class ClientEvent with _$ClientEvent {
  /// Event created upon every app startup, with some data needed
  /// for the engine to work, like personalisation and feed market
  /// (for performing background queries).
  @Implements<SystemClientEvent>()
  @Assert('deConfig == null || deConfig != ""')
  const factory ClientEvent.init(
    Configuration configuration, {
    String? deConfig,
  }) = Init;

  /// Event created when the user changes market or count (nb of items per page)
  /// for the feed ie. in global settings.
  @Implements<SystemClientEvent>()
  @Assert('feedMarkets == null || feedMarkets.length > 0')
  @Assert('maxItemsPerFeedBatch == null || maxItemsPerFeedBatch > 0')
  @Assert('maxItemsPerSearchBatch == null || maxItemsPerSearchBatch > 0')
  const factory ClientEvent.configurationChanged({
    FeedMarkets? feedMarkets,
    int? maxItemsPerFeedBatch,
    int? maxItemsPerSearchBatch,
  }) = ConfigurationChanged;

  /// Event created when opening up discovery screen (upon initial start
  /// of the app or when we are returning to the previously displayed
  /// discovery feed). When restoring the previous feed it returns all the documents
  /// that were still accessible to the user, namely those that weren't closed in
  /// the [FeedDocumentsClosed] event.
  @Implements<FeedClientEvent>()
  const factory ClientEvent.restoreFeedRequested() = RestoreFeedRequested;

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

  /// Event created when the client wants to override both excluded and trusted
  /// [Source]s.
  @Implements<FeedClientEvent>()
  const factory ClientEvent.setSourcesRequested({
    required Set<Source> trustedSources,
    required Set<Source> excludedSources,
  }) = SetSourcesRequested;

  /// Event created when a source is added to the list of excluded sources.
  @Implements<FeedClientEvent>()
  const factory ClientEvent.excludedSourceAdded(Source source) =
      ExcludedSourceAdded;

  /// Event created when a source is removed from the list of excluded sources.
  @Implements<FeedClientEvent>()
  const factory ClientEvent.excludedSourceRemoved(Source source) =
      ExcludedSourceRemoved;

  /// Event created when a client requests for list of excluded sources.
  @Implements<FeedClientEvent>()
  const factory ClientEvent.excludedSourcesListRequested() =
      ExcludedSourcesListRequested;

  /// Event created when a source is added to the list of trusted sources.
  @Implements<FeedClientEvent>()
  const factory ClientEvent.trustedSourceAdded(Source source) =
      TrustedSourceAdded;

  /// Event created when a source is removed from the list of trusted sources.
  @Implements<FeedClientEvent>()
  const factory ClientEvent.trustedSourceRemoved(Source source) =
      TrustedSourceRemoved;

  /// Event created when a client requests for list of trusted sources.
  @Implements<FeedClientEvent>()
  const factory ClientEvent.trustedSourcesListRequested() =
      TrustedSourcesListRequested;

  /// Event created when a client requests for list of available sources.
  @Implements<FeedClientEvent>()
  const factory ClientEvent.availableSourcesListRequested(
    String fuzzySearchTerm,
  ) = AvailableSourcesListRequested;

  /// Event created when a [Document] has been viewed in a certain mode for
  /// the given amount of time in seconds.
  @Implements<DocumentClientEvent>()
  @Assert('seconds >= 0')
  const factory ClientEvent.documentTimeSpent(
    DocumentId documentId,
    DocumentViewMode mode,
    int seconds,
  ) = DocumentTimeSpent;

  /// Event created when the user swipes the [Document] card or clicks a button
  /// to indicate that the document is `positive`, `negative` or `neutral`.
  @Implements<DocumentClientEvent>()
  const factory ClientEvent.userReactionChanged(
    DocumentId documentId,
    UserReaction userReaction,
  ) = UserReactionChanged;

  /// Event created when the user starts a new active search. `searchBy`
  /// determines what `term` is, e.g. [SearchBy.query] indicates that `term` is
  /// a query term.
  @Implements<SearchClientEvent>()
  @Assert('term != ""')
  const factory ClientEvent.activeSearchRequested(
    String term,
    SearchBy searchBy,
  ) = ActiveSearchRequested;

  /// Event created when the client asks for a next batch of documents related
  /// to the current active search.
  @Implements<SearchClientEvent>()
  const factory ClientEvent.nextActiveSearchBatchRequested() =
      NextActiveSearchBatchRequested;

  /// Event created when the user returns to the last active search results.
  @Implements<SearchClientEvent>()
  const factory ClientEvent.restoreActiveSearchRequested() =
      RestoreActiveSearchRequested;

  /// Event created when the client asks for current active search term.
  @Implements<SearchClientEvent>()
  const factory ClientEvent.activeSearchTermRequested() =
      ActiveSearchTermRequested;

  /// Event created when the client makes [Document]s in the active search
  /// not accessible to the user anymore. The engine registers those documents
  /// as immutable, so they can't be changed anymore by the client.
  @Implements<SearchClientEvent>()
  const factory ClientEvent.activeSearchClosed() = ActiveSearchClosed;

  /// Event created when the user starts a new deep search.
  @Implements<SearchClientEvent>()
  const factory ClientEvent.deepSearchRequested(DocumentId id) =
      DeepSearchRequested;

  /// Resets the AI state of the engine.
  ///
  /// This does not reset configurations like markets, excluded and
  /// trusted sources.
  @Implements<SystemClientEvent>()
  const factory ClientEvent.resetAi() = ResetAiRequested;

  /// Converts json Map to [ClientEvent].
  factory ClientEvent.fromJson(Map<String, Object?> json) =>
      _$ClientEventFromJson(json);
}
