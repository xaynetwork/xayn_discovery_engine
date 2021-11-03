import 'package:xayn_discovery_engine/src/api/events/base_events.dart'
    show ClientEvent, EngineEvent;
import 'package:xayn_discovery_engine/src/api/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/search_type.dart'
    show SearchType;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show SearchId;

/// Event created when the user triggers a search query:
///  - by typing the search term aka. "real-time search"
///  - by deep search on a document
///  - by selecting item provided by autosuggestion
///  - by selecting item from history of past searches
///  - by changing the search market
///  - by changing the type of search
class SearchRequested extends ClientEvent {
  final String term;

  /// Search types => web, image, video, news, etc.
  final List<SearchType> types;

  const SearchRequested(this.term, this.types);
}

/// Event created as a successful response to [SearchRequested] event.
///
/// Passes back list of [Document] objects together with [SearchId] to indicate
/// which "search" these objects belong to.
class SearchRequestSucceeded extends EngineEvent {
  final SearchId searchId;
  final List<Document> items;

  const SearchRequestSucceeded(this.searchId, this.items);
}

enum SearchFailureReason {
  notAuthorised,
  notFound,

  /// others
}

/// Event created as a failure response to [SearchRequested] event.
///
/// Passes back a failure reason, that the client can use to determine
/// how to react, ie. display user friendly messages, repeat request, etc.
class SearchRequestFailed extends EngineEvent {
  final SearchFailureReason reason;

  const SearchRequestFailed(this.reason);
}

/// Event created when the user triggers a request for next batch
/// of the current search, usually by scrolling to the end of the results list.
class NextSearchBatchRequested extends ClientEvent {
  final SearchId searchId;

  const NextSearchBatchRequested(this.searchId);
}

/// Event created as a successful response to [NextSearchBatchRequested] event.
///
/// Passes back list of [Document] objects for the next page/batch.
class NextSearchBatchRequestSucceeded extends EngineEvent {
  final List<Document> items;

  const NextSearchBatchRequestSucceeded(this.items);
}

/// Event created as a failure response to [NextSearchBatchRequested] event.
///
/// Passes back a failure reason, that the client can use to determine
/// how to react, ie. display user friendly messages, repeat request, etc.
class NextSearchBatchRequestFailed extends EngineEvent {
  /// combined enum for `SearchRequestFailed` and `NextSearchBatchRequestFailed`
  final SearchFailureReason reason;

  const NextSearchBatchRequestFailed(this.reason);
}

/// Event created when we want to restore a previous search state. The engine
/// will respond with all related `Documents` fetched and reranked
/// on the previous app "run".
///
/// The `searchId` needs to remain "stable" from the app point of view. When
/// restoring previous search the engine should give back all the documents
/// that are related to that `searchId`.
///
/// When asking for a next page of results for that "old" query, but during
/// a "new" session, the engine needs to send back documents that are related
/// to the same "old" `searchId`.
class SearchRestoreRequested extends ClientEvent {
  final SearchId searchId;

  const SearchRestoreRequested(this.searchId);
}

/// Event created as a successful response to [SearchRestoreRequested] event.
///
/// Passes back list of all [Document] objects related to previously performed
/// search that the client requested to restore.
class SearchRestoreRequestSucceeded extends EngineEvent {
  final List<Document> items;

  const SearchRestoreRequestSucceeded(this.items);
}

enum SearchRestoreFailureReason {
  notFound,
  searchClosed,

  /// TODO: add other reasons
}

/// Event created as a failure response to [SearchRestoreRequested] event.
///
/// Passes back a failure reason, that the client can use to determine
/// how to react, ie. display user friendly messages, repeat request, etc.
class SearchRestoreRequestFailed extends EngineEvent {
  final SearchRestoreFailureReason reason;

  const SearchRestoreRequestFailed(this.reason);
}

/// Event created when the client wants to know which searches
/// the discovery engine can restore.
class ActiveSearchesRequested extends ClientEvent {
  const ActiveSearchesRequested();
}

/// Event created as a successful response to [ActiveSearchesRequested] event.
///
/// Passes back list of all [SearchId] objects that the client can then request
/// to restore.
class ActiveSearchesRequestSucceeded extends EngineEvent {
  final Set<SearchId> searchIds;

  const ActiveSearchesRequestSucceeded(this.searchIds);
}

enum ActiveSearchesFailure {
  notFound,
  // etc.
}

/// Event created as a failure response to [ActiveSearchesRequested] event.
///
/// Passes back a failure reason, that the client can use to determine
/// how to react, ie. display user friendly messages, repeat request, etc.
class ActiveSearchesRequestFailed extends EngineEvent {
  final ActiveSearchesFailure reason;

  const ActiveSearchesRequestFailed(this.reason);
}

/// Event created when a search and related Documents can't be accessed again
/// by the user from the UI. Usualy it happens when the users closes a tab/tabs,
/// so all searches within these tabs are also closed.
class SearchesClosed extends ClientEvent {
  final Set<SearchId> searchIds;

  const SearchesClosed(this.searchIds);
}
