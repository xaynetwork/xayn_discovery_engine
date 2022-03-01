# Events

As we are already sending events on mobile between the UI thread and the isolate, and because we plan to move the discovery engine to a web worker, we are thinking about introducing event-driven approach for communicating across these boundaries.

Below is a list of proposed events needed by our system.

Some rules for events:
 - every event needs to be serializable, so that we can send it through any boundary (isolate, web worker, network, etc.).
 - every event that needs a response will contain also auto-generated `ID` which the response event would contain to match with the request.

## Event base classes
`Event` base class should be different for events sent from the app and events sent from the discovery engine.

```dart
/// Base for all event classes
abstract class Event {}

/// For events sent from the app to the engine
abstract class ClientEvent extends Event {}

/// For events sent from the engine to the app
abstract class EngineEvent extends Event {}
```


### Init

Event created upon every app startup, with some data needed for the engine to work, like personalisation and feed market (for performing background queries).

```dart
class Configuration {
  final String apiKey;
  final String apiBaseUrl;
  final String feedMarket;
  final String searchMarket;
  final int maxItemsPerSearchPage;
  final int maxItemsPerFeedPage;
  final String applicationDirectoryPath;
}

class Init extends ClientEvent {
  final bool isPersonalisationOn;
  final Configuration config;

  const Init(this.isPersonalisationOn, this.config);
}
```

### ConfigurationChanged

Event created when the user changes market for the feed ie. in global settings or changes some parameters for search, like market or count (nb of items per page).

```dart
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
```

### RestoreFeedRequested

Event created when opening up discovery screen (upon initial start of the app or when we are returning to previously displayed discovery feed). When restoring previous feed it returns all the documents, that were still accessible to the user, so they weren't closed by the `FeedDocumentsClosed` event.

```dart
class RestoreFeedRequested extends ClientEvent {
  const RestoreFeedRequested();
}

class RestoreFeedSucceeded extends EngineEvent {
  final List<Document> items;

  const RestoreFeedSucceeded(this.items);
}

enum FeedFailureReason {
  notAuthorised,
  noNewsForMarket,
  // etc.
}

class RestoreFeedFailed extends EngineEvent {
  /// Error code that frontend can use to determine how to react,
  /// ie. display user friendly messages, repeat request, etc.
  /// It could also be dedicated classes/exceptions etc.
  final FeedFailureReason reason;

  const RestoreFeedFailed(this.reason);
}
```

### NextFeedBatchRequested

Event created when the app wants to requests new content for the discovery feed:
 - when reaching the end of the current list of items
 - in response to `NewFeedAvailable` event, or after deliberate user action like pressing the button to fetch new items
 - on some time trigger
 - as a follow up when changing the news market

```dart
class NextFeedBatchRequested extends ClientEvent {
  const NextFeedBatchRequested();
}

class NextFeedBatchRequestSucceeded extends EngineEvent {
  final List<Document> items;

  const NextFeedBatchRequestSucceeded(this.items);
}

class NextFeedBatchRequestFailed extends EngineEvent {
  /// combined enum for `NewRequestFailed` and `NextFeedBatchRequestFailed`
  final FeedFailureReason reason;
  
  const NextFeedBatchRequestFailed(this.reason);
}
```

### NewFeedAvailable

Event created by the engine, possibly after doing some background queries to let the app know that there is new content available for the discovery feed. In response to that event the app may decide to show an indicator for the user that new content is ready or it might send `RestoreFeedRequested` event to ask for new documents.

```dart
class NewFeedAvailable extends EngineEvent {
  const NewFeedAvailable();
}
```

### SearchRequested

Event created when the user triggers a search query:
 - by typing the search term aka. "real-time search"
 - by deep search on a document
 - by selecting item provided by autosuggestion
 - by selecting item from history of past searches
 - by changing the search market

```dart

class SearchRequested extends ClientEvent {
  final String term;

  const SearchRequested(this.term);
}

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

class SearchRequestFailed extends EngineEvent {
  final SearchFailureReason reason;

  const SearchRequestFailed(this.reason);
}
```

### NextSearchBatchRequested

Event created when the user triggers a request for next batch of the current search, usually by scrolling to the end of the list of results.

```dart
class NextSearchBatchRequested extends ClientEvent {
  final SearchId searchId;

  const NextSearchBatchRequested(this.searchId);
}

class NextSearchBatchRequestSucceeded extends EngineEvent {
  final List<Document> items;

  const NextSearchBatchRequestSucceeded(this.items);
}

class NextSearchBatchRequestFailed extends EngineEvent {
  /// combined enum for `SearchRequestFailed` and `NextSearchBatchRequestFailed`
  final SearchFailureReason reason;

  const NextSearchBatchRequestFailed(this.reason);
}
```

### SearchRestoreRequested

Event created when we want to restore a previous search state. The engine will respond with all related `Documents` fetched and reranked on the previous app "run".

> The `searchId` needs to remain "stable" from the app point of view. When restoring previous search the engine should give back all the documents that are related to that `searchId`.
>
> When asking for a next page of results for that "old" query, but during a "new" session, the engine needs to send back documents that are related to the same "old" `searchId`.

```dart
class SearchRestoreRequested extends ClientEvent {
  final SearchId searchId;

  const SearchRestoreRequested(this.searchId);
}

class SearchRestoreRequestSucceeded extends EngineEvent {
  final List<Document> items;

  const SearchRestoreRequestSucceeded(this.items);
}


enum SearchRestoreFailureReason {
  notFound,
  searchClosed,
  /// TODO: add other reasons
}

class SearchRestoreRequestFailed extends EngineEvent {
  final SearchRestoreFailureReason reason;

  const SearchRestoreRequestFailed(this.reason);
}
```

### ActiveSearchesRequested

Event created when the client wants to know which searches the engine can restore.

```dart
class ActiveSearchesRequested extends ClientEvent {
  const ActiveSearchesRequested();
}

class ActiveSearchesRequestSucceeded extends EngineEvent {
  final Set<SearchId> searchIds;

  const ActiveSearchesRequestSucceeded(this.searchIds);
}

enum ActiveSearchesFailure {
  notFound,
  // etc.
}

class ActiveSearchesRequestFailed extends EngineEvent {
  final ActiveSearchesFailure reason;

  const ActiveSearchesRequestFailed(this.reason);
}
```

### SearchesClosed

Event created when a search and related Documents can't be accessed again by the user from the UI. Usualy it happens when the users closes a tab/tabs, so all searches within these tabs are also closed.

```dart
class SearchesClosed extends ClientEvent {
  final Set<SearchId> searchIds;

  const SearchesClosed(this.searchIds);
}
```

### FeedDocumentsClosed

Event created when the client makes Documents in the feed not accessible to the user anymore. The engine registers those documents as immutable, so they can't be changed anymore by the client.

```dart
class FeedDocumentsClosed extends ClientEvent {
  final Set<DocumentId> documentIds;

  const FeedDocumentsClosed(this.documentIds);
}
```

### DocumentStatusChanged

Event created when the `DocumentStatus` changed:
 - when the document was presented to the user the status changes from `missed` to `presented`.
 - when the document was presented but then was scrolled out of the screen the status changes from `presented` to `skipped`. It means the user saw the document, but it wasn't relevant.
 - when the document was opened the status changes from `presented` or `skipped` to `opened`. It means the user was interested enough in the document to open it.

```dart
class DocumentStatusChanged extends ClientEvent {
  final DocumentId documentId;
  final DocumentStatus status;

  const DocumentStatusChanged(this.documentId, this.status);
}
```

### UrlOpened

Same as `DocumentStatusChanged` with `DocumentStatus.opened` but for pages in the webview that didn't originate from a list of documents:
- opened an external url, from a different app
- opened as a direct url, by typing it in the search field
- navigated to inside of the webview, after clicking on a link

The engine responds to that event with `DocumentFromUrlCreated` which contains `documentId` to be used with other "document" events, like `DocumentClosed`, `UserReactionChanged`, etc.

```dart
// the app sends this event after accessing at least title,
// would be good if snippet was there too
// alternatively we could call it `DocumentFromUrlRequested`
// but it's a more generic name, and it doesn't contain
// the information that document was also "opened"
class UrlOpened extends ClientEvent {
  final String url;
  final String title;
  final String snippet;

  const UrlOpened(this.url, this.title, this.snippet);
}

class DocumentFromUrlCreated extends EngineEvent {
  final DocumentId documentId;

  const DocumentFromUrlCreated(this.documentId);
}
```

### DocumentClosed

Event created when the document was closed, either by going back to documents list or by navigating further to a link contained by the document. It helps to calculate how much time user spent reviewing the document.
 
For cases when the user will open and close the same document multiple times (for the same search), the engine should store and use only the maximum time spent by the user on a document.

```dart
class DocumentClosed extends ClientEvent {
  final DocumentId documentId;

  const DocumentClosed(this.documentId);
}
```


### UserReactionChanged

Event created when the user swipes the document card or clicks a button to indicate that the document is `positive`, `negative` or `neutral`.

```dart
class UserReactionChanged extends ClientEvent {
  final DocumentId documentId;
  final UserReaction reaction;

  const UserReactionChanged(this.documentId, this.reaction);
}
```

### BookmarkCreated

Event created when the user bookmarks a document. Engine internally could treat it as `like`.

```dart
class BookmarkCreated extends ClientEvent {
  final DocumentId documentId;

  const BookmarkCreated(this.documentId);
}
```

### BookmarksRemoved

Event created when the user removed single or multiple bookmarks. Engine internally could treat it as `neutral`.

```dart
class BookmarksRemoved extends ClientEvent {
  final Set<DocumentId> documentIds;
  
  const BookmarksRemoved(this.documentIds);
}
```

### ContentCategoriesDismissed

Event created when the user dismisses categories/topics when doing a "negative" swipe, ie. on item in the news feed.

```dart
class ContentCategoriesDismissed extends ClientEvent {
  final DocumentId documentId;
  final Set<String> categories;

  const ContentCategoriesDismissed(this.documentId, this.categories);
}
```

### ContentCategoriesAccepted

Event created when the user removes "ban" from previously dismisses feed categories/topics.

```dart
class ContentCategoriesAccepted extends ClientEvent {
  final Set<String> categories;

  const ContentCategoriesAccepted(this.categories);
}
```

### ClientEventSucceeded

Event created to inform the client that a particular "fire and forget" event, like ie. `UserReactionChanged`, was successfuly processed by the engine.

```dart
class ClientEventSucceeded extends EngineEvent {
  final EventId eventId;

  const ClientEventSucceeded(this.eventId);
}
```

### EngineExceptionRaised

Event created by the engine for multitude of generic reasons, also as a "failure" event in response to "fire and forget" events, like ie. `UserReactionChanged`.

```dart
enum EngineExceptionReason {
  engineNotReady,
  // other possible errors will be added below
}

class EngineExceptionRaised extends EngineEvent {
  final EngineExceptionReason reason;

  const EngineExceptionRaised(this.reason);
}
```
