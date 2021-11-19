import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/api/events/client_groups.dart'
    show ClientEvent;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';

part 'feed_events.freezed.dart';
part 'feed_events.g.dart';

@freezed
class FeedClientEvent with _$FeedClientEvent implements ClientEvent {
  /// Event created when opening up discovery screen (upon initial start
  /// of the app or when we are returning to previously displayed
  /// discovery feed). When restoring previous feed it returns all the documents,
  /// that were still accessible to the user, so they weren't closed by
  /// the [FeedDocumentsClosed] event.
  const factory FeedClientEvent.feedRequested() = FeedRequested;

  /// Event created when the app wants to requests new content
  /// for the discovery feed:
  ///  - when reaching the end of the current list of items
  ///  - in response to `NewFeedAvailable` event, or after deliberate user action
  /// like pressing the button to fetch new items
  ///  - on some time trigger
  ///  - as a follow up when changing the news market
  const factory FeedClientEvent.newFeedRequested() = NewFeedRequested;

  /// Event created when the client makes `Documents` in the feed not accessible
  /// to the user anymore. The engine registers those documents as immutable,
  /// so they can't be changed anymore by the client.
  const factory FeedClientEvent.feedDocumentsClosed(
    Set<DocumentId> documentIds,
  ) = FeedDocumentsClosed;

  /// Event created when the user dismisses categories/topics when doing
  /// a "negative" swipe, ie. on item in the news feed.
  const factory FeedClientEvent.contentCategoriesDismissed(
    DocumentId documentId,
    Set<String> categories,
  ) = ContentCategoriesDismissed;

  /// Event created when the user removes "ban" from previously dismisses feed
  /// categories/topics.
  const factory FeedClientEvent.contentCategoriesAccepted(
    Set<String> categories,
  ) = ContentCategoriesAccepted;

  /// Converts json Map to [FeedClientEvent].
  factory FeedClientEvent.fromJson(Map<String, dynamic> json) =>
      _$FeedClientEventFromJson(json);
}
