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
class Init extends ClientEvent {
  final bool isPersonalisationOn;
  final String market;

  const Init(this.isPersonalisationOn, this.market);
}
```

### ResetEngine

Event created when the app decides to reset the AI (start fresh).

```dart
class ResetEngine extends ClientEvent {
  const ResetEngine();
}
```

### PersonalizationChanged

Events created when the user toggles the AI on/off.

When the personalisation is OFF:
- we are still reranking all the incoming results, but we don't use personal data to do it
- we are preventing storing queries and documents in the history, and sending/processing document-related events (likes, dislikes, opened, closed)

Every document gets a rank from the reranker only once. When we toggle we switch between the API rank and Engine rank.

```dart
class PersonalizationChanged extends ClientEvent {
  final bool isOn;

  const PersonalizationChanged(this.isOn);
}
```

### FeedMarketChanged

Event created when the user changes market for the feed, ie. in global settings.

```dart
class FeedMarketChanged extends ClientEvent {
  final String market;
}
```

### FeedRequested

Event created when the app requests content for the discovery feed:
 - upon initial start of the app
 - on certain predefined triggers like time-interval, entering `DiscoveryScreen`, etc.
 - as a follow up when changing the news market

```dart
class FeedRequested extends ClientEvent {}

class FeedRequestSucceeded extends EngineEvent {
  final List<Document> items;

  const FeedRequestSucceeded(this.items);
}

class FeedRequestFailed extends EngineEvent {
  /// Error code that frontend can use to display user friendly messages.
  /// It could also be of type `String`, `enum`, etc.
  final int reason;
}
```

### NewFeedAvailable

Event created by the engine, possibly after doing some background queries to let the app know that there is new content available for the discovery feed. In response to that event the app may decide to show an indicator for the user that new content is ready or it might send `FeedRequested` event to ask for new documents.

```dart
class NewFeedAvailable extends EngineEvent {}
```

### FeedRestoreRequested

Event created when we are returning to previously displayed discovery feed.
> Q: What is the user expectation? Maybe we should load new feed every session instead of what was shown before?

```dart
class FeedRestoreRequested extends ClientEvent {
  const FeedRestoreRequested();
}

class FeedRestoreSucceeded extends EngineEvent {
  final List<Document> items;

  const FeedRestoreSucceeded(this.items);
}

class FeedRestoreFailed extends EngineEvent {
  final int reason;

  const FeedRestoreFailed(this.reason);
}

```

### SearchRequested

Event created when the user triggers a search query:
 - by typing the search term aka. "real-time search"
 - by deep search on a document
 - by selecting item provided by autosuggestion
 - by selecting item from history of past searches
 - by changing the search market
 - by changing the type of search

```dart
class SearchRequested extends ClientEvent {
  final String term;
  /// Search types => web, image, video, news, etc.
  final List<SearchType> type;
  final String market;
}

class SearchRequestSucceeded extends EngineEvent {
  final List<Document> items;
  final SearchId searchId;
}

class SearchRequestFailed extends EngineEvent {
  /// Error code that frontend can use to display user friendly messages.
  /// It could also be of type `String`, `enum`, etc.
  final int reason;
}
```

### NextSearchBatchRequested

Event created when the user triggers a request for next batch of the current search, usually by scrolling to the end of the list of results.

```dart
class NextSearchBatchRequested extends ClientEvent {
  final SearchId searchId;
}

class NextSearchBatchRequestSucceeded extends EngineEvent {
  final List<Document> items;
}

class NextSearchBatchRequestFailed extends EngineEvent {
  /// Error code that frontend can use to display user friendly messages.
  /// It could also be of type `String`, `enum`, etc.
  final int reason;
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
}

class SearchRestoreRequestSucceeded extends EngineEvent {
  final List<Document> items;
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

The engine responds to that event with `DocumentFromUrlCreated` which contains `documentId` to be used with other "document" events, like `DocumentClosed`, `DocumentFeedbackChanged`, etc.

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


### DocumentFeedbackChanged

Event created when the user swipes the document card or clicks a button to indicate that the document is `relevant`, `irrelevant` or `neutral`.

```dart
class DocumentFeedbackChanged extends ClientEvent {
  final DocumentId documentId;
  final DocumentFeedback feedback;

  const DocumentFeedbackChanged(this.documentId, this.feedback);
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

### SearchesClosed

Event created when a search and related Documents can't be accessed again by the user from the UI. Usualy it happens when the users closes a tab/tabs, so all searches within these tabs are also closed.

```dart
class SearchesClosed extends ClientEvent {
  final Set<DocumentId> searchIds;

  const SearchesClosed(this.searchIds);
}
```

### ContentCategoriesDismissed

Event created when the user dismisses categories/topics when doing a "negative" swipe, ie. on item in the news feed.

```dart
class ContentCategoriesDismissed extends ClientEvent {
  final DocumentId documentId;
  final Set<String> categories;
}
```

### ContentCategoriesReallowed

Event created when the user removes "ban" from previously dismisses feed categories/topics.

```dart
class ContentCategoriesReallowed extends ClientEvent {
  final Set<String> categories;
}
```

### ActiveSearches

Event created when the client wants to know which searches the engine can restore.

```dart
class ActiveSearchesRequested extends ClientEvent {}

class ActiveSearchesRequestSucceeded extends EngineEvent {
  final Set<SearchId> searchIds;

  const ActiveSearchesRequestSucceeded(this.searchIds);
}

class ActiveSearchesRequestFailed extends EngineEvent {
  // an enum in the real implementation
  final int reason;

  const ActiveSearchesRequestFailed(this.reason);
}
```

### EngineExceptionRaised

Event created by the engine for multitude of generic reasons.

```dart
enum EngineExceptionReason {
  noInitReceived,
  // other possible errors will be added below
}

class EngineExceptionRaised extends EngineEvent {
  final EngineExceptionReason reason;
}
```
