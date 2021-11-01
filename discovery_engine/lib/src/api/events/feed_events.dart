import 'package:xayn_discovery_engine/src/api/events/base_events.dart'
    show ClientEvent, EngineEvent;
import 'package:xayn_discovery_engine/src/api/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;

/// Event created when opening up discovery screen (upon initial start
/// of the app or when we are returning to previously displayed
/// discovery feed). When restoring previous feed it returns all the documents,
/// that were still accessible to the user, so they weren't closed by
/// the [FeedDocumentsClosed] event.
class FeedRequested extends ClientEvent {
  const FeedRequested();
}

/// Event created as a successful response to [FeedRequested] event.
/// Passes back a list of [Document] entities back to the client.
class FeedRequestSucceeded extends EngineEvent {
  final List<Document> items;

  const FeedRequestSucceeded(this.items);
}

enum FeedFailureReason {
  notAuthorised,
  noNewsForMarket,
  // etc.
}

/// Event created as a failure response to [FeedRequested] event.
///
/// Passes back a failure reason, that the client can use to determine
/// how to react, ie. display user friendly messages, repeat request, etc.
class FeedRequestFailed extends EngineEvent {
  final FeedFailureReason reason;

  const FeedRequestFailed(this.reason);
}

/// Event created when the app wants to requests new content
/// for the discovery feed:
///  - when reaching the end of the current list of items
///  - in response to `NewFeedAvailable` event, or after deliberate user action
/// like pressing the button to fetch new items
///  - on some time trigger
///  - as a follow up when changing the news market
class NewFeedRequested extends ClientEvent {
  const NewFeedRequested();
}

/// Event created as a successful response to [NewFeedRequested] event.
/// Passes back a list of [Document] objects back to the client.
class NewFeedRequestSucceeded extends EngineEvent {
  final List<Document> items;

  const NewFeedRequestSucceeded(this.items);
}

/// Event created as a failure response to [NewFeedRequested] event.
///
/// Passes back a failure reason, that the client can use to determine
/// how to react, ie. display user friendly messages, repeat request, etc.
class NewFeedRequestFailed extends EngineEvent {
  final FeedFailureReason reason;

  const NewFeedRequestFailed(this.reason);
}

/// Event created by the engine, possibly after doing some background queries
/// to let the app know that there is new content available for the discovery
/// feed. In response to that event the app may decide to show an indicator
/// for the user that new content is ready or it might send [FeedRequested]
/// event to ask for new documents.
class NewFeedAvailable extends EngineEvent {
  const NewFeedAvailable();
}

/// Event created when the client makes `Documents` in the feed not accessible
/// to the user anymore. The engine registers those documents as immutable,
/// so they can't be changed anymore by the client.
class FeedDocumentsClosed extends ClientEvent {
  final Set<DocumentId> documentIds;

  const FeedDocumentsClosed(this.documentIds);
}
