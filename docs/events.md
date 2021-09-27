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
abstract class AppEvent extends Event {}

/// For events sent from the engine to the app
abstract class EngineEvent extends Event {}
```


### Init

Event created upon every app startup, with some data needed for the engine to work.

```dart
class Init extends AppEvent {
  final bool isPersonalisationOn;

  const Init(this.isPersonalisationOn);
}
```

### ResetEngine

Event created when the app decides to reset the AI (start fresh).

```dart
class ResetEngine extends AppEvent {
  const ResetEngine();
}
```

### PersonalisationToggled

Event created when the user toggles the AI on/off.

When the personalisation is OFF:
- we are still reranking all the incoming results, but we are sorting them with original rank from Bing
- we are preventing storing queries and documents in the history, and sending/processing document-related events (likes, dislikes, opened, closed)

Every document gets a rank from the reranker only once. When we toggle we switch between the API rank and Engine rank.

```dart
class PersonalisationToggled extends AppEvent {
  const PersonalisationToggled();
}

// alternatively
class PersonalisationTurnedOn extends AppEvent {
  const PersonalisationTurnedOn();
}
class PersonalisationTurnedOff extends AppEvent {
  const PersonalisationTurnedOff();
}
```


### FeedRequested

Event created when the app requests contetn for the discovery feed:
 - upon initial start of the app
 - on certain predefined triggers like time-interval, entering `DiscoveryScreen`, etc.
 - when changing the news market

```dart
class FeedRequested extends AppEvent {
  final String market;
}

class FeedRequestSucceded extends EngineEvent {
  final List<Document> items;
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
class FeedRestoreRequested extends AppEvent {
  // do we need to send prev `documentIds`?
  final List<UniqueId> documentIds;

  const FeedRestoreRequested(this.documentIds);
}

class FeedRestoreSucceded extends EngineEvent {
  final List<Document> items;

  const FeedRestoreSucceded(this.documentIds);
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
class SearchRequested extends AppEvent {
  final String term;
  /// Search types => web, image, video, news, etc.
  /// We could think about making this a List<SearchType>
  final SearchType type;
  final String market;
}

class SearchRequestSucceded extends EngineEvent {
  final List<Document> items;
  final bool hasNextPage;
  // Might be full Query if the UI needs it
  final UniqueId queryId;
}

// alternatively we might think about sending different structure
class SearchRequestSucceded extends EngineEvent {
  final Map<UniqueId, Document> itemsById;
  final List<UniqueId> idsOrderedByApi;
  final List<UniqueId> idsOrderedByEngine;
  final bool hasNextPage;
  final UniqueId queryId;
}

class SearchRequestFailed extends EngineEvent {
  /// Error code that frontend can use to display user friendly messages.
  /// It could also be of type `String`, `enum`, etc.
  final int reason;
}
```

### SearchPageRequested

Event created when the user triggers a request for next page of the current search, usually by scrolling to the end of the list of results.

```dart
class SearchPageRequested extends AppEvent {
  final UniqueId queryId;
  final int page;
}

class SearchPageRequestSucceded extends EngineEvent {
  final List<Document> items;
  final bool hasNextPage;
}

class SearchPageRequestFailed extends EngineEvent {
  /// Error code that frontend can use to display user friendly messages.
  /// It could also be of type `String`, `enum`, etc.
  final int reason;
}
```

### SearchRestoreRequested

Event created when we want to restore a previous search state. The enging will respond with `Query` and all related `Documents` fetched and reranked on the previous app "run".

> The `queryId` needs to remain "stable" from the app point of view. When restoring previous search the engine should give back same `Query` (with the same `queryId`) as requested by the app, and all the documents should be contain that `queryId`.
>
> When asking for a next page of results for that "old" `Query`, but during a "new" session, the engine needs to send back documents that are related to the same "old" `queryId`.

```dart
class SearchRestoreRequested extends AppEvent {
  final UniqueId queryId;
}

class SearchRestoreRequestSucceded extends EngineEvent {
  final Query query;
  final List<Document> results;
  final bool hasNextPage;
}
```

### DocumentPresented

Event created when the document was presented to the user. It can only change the `DocumentStatus` from `missed` to `presented`.

```dart
class DocumentPresented extends AppEvent {
  final UniqueId documentId;

  const DocumentPresented(this.documentId);
}
```

### DocumentSkipped

Event created when the document was presented but was scrolled out of the screen. It can only change the `DocumentStatus` from `presented` to `skipped`. It means the user saw the document, but it wasn't relevant.

```dart
class DocumentSkipped extends AppEvent {
  final UniqueId documentId;

  const DocumentSkipped(this.documentId);
}
```

### DocumentOpened

Event created when the document was opened. It can only change the `DocumentStatus` from `presented` or `skipped` to `opened`. It means the user was interested enough in the document to open it.

```dart
class DocumentOpened extends AppEvent {
  final UniqueId documentId;

  const DocumentOpened(this.documentId);
}
```

### UrlOpened

Same as `DocumentOpened` but for pages in the webview that didn't originate from a list of documents:
- opened an external url, from a different app
- opened as a direct url, by typing it in the search field
- navigated to inside of the webview, after clicking on a link

The engine responds to that event with `DocumentFromUrlCreated` which contains `documentId` to be used with other "document" events, like `DocumentClosed`, `DocumentLiked`, etc.

```dart
// the app sends this event after accessing at least title,
// would be good if snippet was there too
// alternatively we could call it `DocumentFromUrlRequested`
// but it's a more generic name, and it doesn't contain
// the information that document was also "opened"
class UrlOpened extends AppEvent {
  final String url;
  final String title;
  final String? snippet;

  const UrlOpened(this.url, this.title, this.snippet);
}

class DocumentFromUrlCreated extends EngineEvent {
  final Document document;
}
```

### DocumentClosed

Event created when the document was closed, either by going back to documents list or by navigating further to a link contained by the document. It helps to calculate how much time user spent reviewing the document.
 
For cases when the user will open and close the same document multiple times (for the same search), the engine should store and use only the maximum time spent by the user on a document.

```dart
class DocumentClosed extends AppEvent {
  final UniqueId documentId;

  const DocumentClosed(this.documentId);
}
```

### DocumentLiked

Event created when the user swipes the document card or clicks a button to indicate that the document is relevant. It should visualy highlight the document in the list.

```dart
class DocumentLiked extends AppEvent {
  final UniqueId documentId;

  const DocumentLiked(this.documentId);
}
```

### DocumentDisliked

Event created when the user swipes the document card or clicks a button to indicate that the document is NOT relevant. It should visualy remove the document from the list.

```dart
class DocumentDisliked extends AppEvent {
  final UniqueId documentId;

  const DocumentDisliked(this.documentId);
}
```

### DocumentNeutral

Event created when `liked` status was reverted OR there was an "undo" action after a `disliked` status change.

```dart
class DocumentNeutral extends AppEvent {
  final UniqueId documentId;

  const DocumentNeutral(this.documentId);
}
```

### DocumentBookmarked

Event created when the user bookmarks a document. Engine internally could treat it as `like`.

```dart
class DocumentBookmarked extends AppEvent {
  final UniqueId documentId;

  const DocumentBookmarked(this.documentId);
}
// alternatively
class BookmarkCreated extends AppEvent {
  final UniqueId documentId;

  const BookmarkCreated(this.documentId);
}
```

### DocumentUnbookmarked

Event created when the user removed a bookmark from the document. Engine internally could treat it as `neutral`.

```dart
class DocumentUnbookmarked extends AppEvent {
  final UniqueId documentId;

  const DocumentUnbookmarked(this.documentId);
}
// alternatively (cause we might remove multiple bookmarks at once)
class BookmarksRemoved extends AppEvent {
  final Set<UniqueId> documentIds;
  
  const BookmarksRemoved(this.documentIds);
}
```

### QueriesClosed

Event created when a search (Query + related Documents) can't be accessed again by the user from the UI. Usualy it happens when the users closes a tab/tabs, so all searches within these tabs are also closed.

```dart
class QueriesClosed extends AppEvent {
  final List<UniqueId> queryIds;

  const QueriesClosed(this.queryIds);
}
```

### FeedCategoriesDismissed

Event created when the user dismisses categories/topics when doing a "negative" swipe on news item in the feed.

```dart
class FeedCategoriesDismissed extends AppEvent {
  final UniqueId documentId;
  final Set<String> categories;
}
```

### FeedCategoriesReallowed

Event created when the user removes "ban" from previously dismisses feed categories/topics.

```dart
class FeedCategoriesReallowed extends AppEvent {
  final Set<String> categories;
}
```
