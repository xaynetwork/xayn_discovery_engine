# Events

As we are already sending events on mobile between the UI thread and the isolate, and because we plan to move the discovery engine to a web worker, we are thinking about introducing event-driven approach for communicating across these boundries.

Below is a list of proposed events needed by our system.

Some rules for events:
 - every event needs to be serializable, so that we can send it through any boundry (isolate, web worker, network, etc.).
 - every event that needs a response will contain also auto-generated `ID` which the response event would use to match with the request.

### SessionStarted

tbd.

```dart
class SessionStarted extends Event {
  const SessionStarted();
}
```

### PersonalisationToggled

Event created when the user toggles the AI on/off.

tbd.

```dart
class PersonalisationToggled extends Event {
  const PersonalisationToggled();
}
```


### NewsFeedRequested

Event created when the app requests contetn for the discovery feed:
 - upon initial start of the app
 - on certain predefined triggers like time-interval, entering `DiscoveryScreen`, etc.
 - when changing the news market

```dart
class NewsFeedRequested extends Event {
  final String market;
}

class NewsFeedRequestSucceded extends Event {
  final List<Document> items;
}

class NewsFeedRequestFailed extends Event {
  /// Error code that frontend can use to display user friendly messages.
  /// It could also be of type `String`, `enum`, etc.
  final int reason;
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
class SearchRequested extends Event {
  final String term;
  /// Search types => web, image, video, news, etc.
  /// We could think about making this a List<SearchType>
  final SearchType type;
  final String market;
}

class SearchRequestSucceded extends Event {
  final List<Document> items;
  final bool hasNextPage;

  /// Query object created by the engine for UI to be able to restore state
  /// and load next pages using `ID` of the Query
  /// Maybe a `queryId` could be enough.
  final Query query;
}

class SearchRequestFailed extends Event {
  /// Error code that frontend can use to display user friendly messages.
  /// It could also be of type `String`, `enum`, etc.
  final int reason;
}
```

### SearchPageRequested

Event created when the user triggers a request for next page of the current search, usually by scrolling to the end of the list of results.

```dart
class SearchPageRequested extends Event {
  final UniqueId queryId;
  final int page;
}

class SearchPageRequestSucceded extends Event {
  final List<Document> items;
  final bool hasNextPage;
}

class SearchPageRequestFailed extends Event {
  /// Error code that frontend can use to display user friendly messages.
  /// It could also be of type `String`, `enum`, etc.
  final int reason;
}
```

### DocumentPresented

Event created when the document was presented to the user. It can only change the `DocumentStatus` from `missed` to `presented`.

```dart
class DocumentPresented extends Event {
  final UniqueId documentId;

  const DocumentPresented(this.documentId);
}
```

### DocumentSkipped

Event created when the document was presented but was scrolled out of the screen. It can only change the `DocumentStatus` from `presented` to `skipped`. It means the user saw the document, but it wasn't relevant.

```dart
class DocumentSkipped extends Event {
  final UniqueId documentId;

  const DocumentSkipped(this.documentId);
}
```

### DocumentOpened

Event created when the document was opened. It can only change the `DocumentStatus` from `presented` or `skipped` to `opened`. It means the user was interested enough in the document to open it.

```dart
class DocumentOpened extends Event {
  final UniqueId documentId;

  const DocumentOpened(this.documentId);
}
```

### DocumentClosed

Event created when the document was closed, either by going back to documents list or by navigating further to a link contained by the document. It helps to calculate how much time user spent reviewing the document.

```dart
class DocumentClosed extends Event {
  final UniqueId documentId;

  const DocumentClosed(this.documentId);
}
```

### DocumentLiked

Event created when the user swipes the document card or clicks a button to indicate that the document is relevant. It should visualy highlight the document in the list.

```dart
class DocumentLiked extends Event {
  final UniqueId documentId;

  const DocumentLiked(this.documentId);
}
```

### DocumentDisliked

Event created when the user swipes the document card or clicks a button to indicate that the document is NOT relevant. It should visualy remove the document from the list.

```dart
class DocumentDisliked extends Event {
  final UniqueId documentId;

  const DocumentDisliked(this.documentId);
}
```

### DocumentNeutral

Event created when `liked` status was reverted OR there was an "undo" action after a `disliked` status change.

```dart
class DocumentNeutral extends Event {
  final UniqueId documentId;

  const DocumentNeutral(this.documentId);
}
```

## Potential events

If we decide to have sentiment for every website loaded in the webview, we need to handle cases when the user:
- opens an external url, from a different app
- opens a url directly, by typing it in the search field
- navigates inside the webview to a different webpage, by clicking on a link

### UrlOpened

Same as `DocumentOpened` but for pages in the webview that didn't originate from a list of documents.

```dart
class UrlOpened extends Event {
  final String url;

  const UrlOpened(this.url);
}
```
### UrlClosed

Same as `DocumentClosed` but for pages in the webview that didn't originate from a list of documents.

```dart
class UrlClosed extends Event {
  final String url;

  const UrlClosed(this.url);
}
```
### UrlLiked

Same as `DocumentLiked` but for pages in the webview that didn't originate from a list of documents.

```dart
class UrlLiked extends Event {
  final String url;

  const UrlLiked(this.url);
}
```
### UrlDisliked

Same as `DocumentDisliked` but for pages in the webview that didn't originate from a list of documents.

```dart
class UrlDisliked extends Event {
  final String url;

  const UrlDisliked(this.url);
}
```
### UrlNeutral

Same as `DocumentNeutral` but for pages in the webview that didn't originate from a list of documents.

```dart
class UrlNeutral extends Event {
  final String url;

  const UrlNeutral(this.url);
}
```


### Open Questions

1. What is a session?
    - when we are restoring previous tab, do we come back to previous session?
    - can search & documents belong to many sessions?

1. Should a `Document` in discovery feed be related to a `Query`?

1. Should the app be able to create `Document` instances or should this be responsibility of the discovery engine?

1. What should happen when toggling personalisation?
  
    The current behaviour is a combination of: 
    - disabling reranking, and
    - incognito mode which prevents storing the history (app history & AI history)

1. Where should we manage and persist personalisation state?

    Based on the personalisation state some events should be disregarded from processing. For example a `DocumentLiked` shouldn't be allowed. The UI will probably never send this event when personalisation is off. But as we shoudn't trust any frontend, the engine should know what is the current state of that flag.

1. How to track sentiment for urls in Webview?

    Do we create documents internaly for all the url related events (`UrlOpened`, `UrlLiked`, etc.). Or do we let the app to create a document and reuse document related events, like `DocumentLiked`, `DocumentOpened`, etc. 
    
    We could also send `UrlOpened` event from the app, for which discovery engine could respond with `DocumentCreatedFromUrl` event, and then switch to "document related events".

1. Do we want to track if a document/url gets bookmarked?

    From AI point of view it could be relevant to know if user finds something worth bookmarking.
