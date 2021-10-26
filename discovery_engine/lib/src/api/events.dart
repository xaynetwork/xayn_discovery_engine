import 'models/configuration.dart';
import 'models/document.dart';
import 'models/search_type.dart';
import 'models/unique_id.dart';

/// Base for all event classes
abstract class Event {
  const Event();
}

/// For events sent from the app to the engine
abstract class ClientEvent extends Event {
  const ClientEvent();
}

/// For events sent from the engine to the app
abstract class EngineEvent extends Event {
  const EngineEvent();
}

/// Event created upon every app startup, with some data needed
/// for the engine to work, like personalisation and feed market
/// (for performing background queries).
class Init extends ClientEvent {
  final bool isPersonalisationOn;
  final Configuration config;

  const Init(this.isPersonalisationOn, this.config);
}

/// Event created when the app decides to reset the AI (start fresh).
class ResetEngine extends ClientEvent {
  const ResetEngine();
}

/// Event created when the user toggles the AI on/off.
///
/// When the personalisation is OFF:
///  - we are still reranking all the incoming results, but we don't use
/// personal data to do it
///  - we are preventing storing queries and documents in the history,
/// and sending/processing document-related events (likes, dislikes, etc.)
///
/// Every document gets a rank from the reranker only once. When we toggle
/// we switch between the API rank and Engine rank.
class PersonalizationChanged extends ClientEvent {
  final bool isOn;

  const PersonalizationChanged(this.isOn);
}

/// Event created when the user changes market for the feed ie.
/// in global settings or changes some parameters for search,
/// like market or count (nb of items per page).
class ConfigurationChanged extends ClientEvent {
  final String? feedMarket;
  final String? searchMarket;
  final int? maxItemsPerSearchPage;
  final int? maxItemsPerFeedPage;

  const ConfigurationChanged({
    this.feedMarket,
    this.searchMarket,
    this.maxItemsPerSearchPage,
    this.maxItemsPerFeedPage,
  });
}

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

/// Event created as a failure response to [NextSearchBatchRequest] event.
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

/// Event created when the client makes `Documents` in the feed not accessible
/// to the user anymore. The engine registers those documents as immutable,
/// so they can't be changed anymore by the client.
class FeedDocumentsClosed extends ClientEvent {
  final Set<DocumentId> documentIds;

  const FeedDocumentsClosed(this.documentIds);
}

/// Event created when the `DocumentStatus` changed:
/// - when the document was presented to the user the status changes
/// from `missed` to `presented`.
/// - when the document was presented but then was scrolled out of the screen
/// the status changes from `presented` to `skipped`. It means the user saw
/// the document, but it wasn't relevant.
/// - when the document was opened the status changes from `presented` or
/// `skipped` to `opened`. It means the user was interested enough in
/// the document to open it.
class DocumentStatusChanged extends ClientEvent {
  final DocumentId documentId;
  final DocumentStatus status;

  const DocumentStatusChanged(this.documentId, this.status);
}

/// Same as [DocumentStatusChanged] with `DocumentStatus.opened` but for pages
/// in the webview that didn't originate from a list of documents:
/// - opened an external url, from a different app
/// - opened as a direct url, by typing it in the search field
/// - navigated to inside of the webview, after clicking on a link
class UrlOpened extends ClientEvent {
  final String url;
  final String title;
  final String snippet;

  const UrlOpened(this.url, this.title, this.snippet);
}

/// Event created as a response to [UrlOpened] event which
/// contains [DocumentId] to be used with other "document" events,
/// like [DocumentClosed], [DocumentFeedbackChanged], etc.
class DocumentFromUrlCreated extends EngineEvent {
  final DocumentId documentId;

  const DocumentFromUrlCreated(this.documentId);
}

/// Event created when the document was closed, either by going back to
/// documents list or by navigating further to a link contained by the document.
/// It helps to calculate how much time user spent reviewing the document.
///
/// For cases when the user will open and close the same document multiple
/// times (for the same search), the engine should store and use only
/// the maximum time spent by the user on a document.
class DocumentClosed extends ClientEvent {
  final DocumentId documentId;

  const DocumentClosed(this.documentId);
}

/// Event created when the user swipes the [Document] card or clicks a button
/// to indicate that the document is `positive`, `negative` or `neutral`.
class DocumentFeedbackChanged extends ClientEvent {
  final DocumentId documentId;
  final DocumentFeedback feedback;

  const DocumentFeedbackChanged(this.documentId, this.feedback);
}

/// Event created when the user bookmarks a document. Engine internally could
/// treat it as `like`.
class BookmarkCreated extends ClientEvent {
  final DocumentId documentId;

  const BookmarkCreated(this.documentId);
}

/// Event created when the user removed single or multiple bookmarks. Engine
/// internally could treat it as `neutral`.
class BookmarksRemoved extends ClientEvent {
  final Set<DocumentId> documentIds;

  const BookmarksRemoved(this.documentIds);
}

/// Event created when the user dismisses categories/topics when doing
/// a "negative" swipe, ie. on item in the news feed.
class ContentCategoriesDismissed extends ClientEvent {
  final DocumentId documentId;
  final Set<String> categories;

  const ContentCategoriesDismissed(this.documentId, this.categories);
}

/// Event created when the user removes "ban" from previously dismisses feed
/// categories/topics.
class ContentCategoriesAccepted extends ClientEvent {
  final Set<String> categories;

  const ContentCategoriesAccepted(this.categories);
}

/// Event created to inform the client that a particular "fire and forget"
/// event, like ie. [DocumentFeedbackChanged], was successfuly processed
/// by the engine.
class ClientEventSucceeded extends EngineEvent {
  const ClientEventSucceeded();
}

enum EngineExceptionReason {
  noInitReceived,
  // other possible errors will be added below
}

/// Event created by the engine for multitude of generic reasons, also
/// as a "failure" event in response to "fire and forget" events, like
/// ie. [DocumentFeedbackChanged].
class EngineExceptionRaised extends EngineEvent {
  final EngineExceptionReason reason;

  const EngineExceptionRaised(this.reason);
}
