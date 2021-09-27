# Document model

`Document` is the model exposed to the Xayn app for representing items in the discovery feed or in the search result list.

```dart
class Document {
  /// Unique identifier of a document
  final UniqueId documentId;

  /// TODO: Do we need to have a relation to the Query?
  /// What about a Document in discovery feed?
  final UniqueId queryId;

  /// Contains all data from search API that are needed for the UI.
  /// It's a base class for web, news, video, image resources.
  final WebResource webResource;

  // These 2 fields can be private cause the UI 
  // only needs to know derived state:
  //  - if document is relevant or irrelevant
  //  - if document was opened
  final DocumentSentiment _sentiment;
  final DocumentStatus _status;

  bool get isRelevant => _sentiment => DocumentSentiment.liked;
  bool get isNotRelevant => _sentiment == DocumentSentiment.disliked;
  bool get wasOpened => _status == DocumentStatus.opened;

  // These 2 fields below will be used to sort documents
  // based on current personalisation state
  final int _apiRank;
  final int _engineRank;

  int get rank(bool isPersonalisationOn) => isPersonalisationOn
    ? _engineRank 
    : _apiRank;

  Document._({
    required this.id,
    required this.queryId,
    required this.webResource,
    DocumentSentiment sentiment = DocumentSentiment.neutral,
    DocumentStatus status = DocumentStatus.missed,
  }) : _sentiment = sentiment,
       _status = status;
}
```

## Attributes of a Document

### Document sentiment

Indicates if the user `liked` or `disliked` the document.

```dart
enum DocumentSentiment {
  neutral,
  liked,
  disliked,
}

```

### Document status

The status of the document is not exposed to the Xayn app, because it has no usage in the UI (apart from `opened` status which could be used to indicate that the document was "visited", which is visually represented by decreased opacity of the document list item).

It indicates what is the document status in context of other documents.

- Every document starts with `missed`, which means the user didn't have the chance to see it.
- When a document is displayed to the user its status is updated to `presented`.
- When the user decides to read it, the status is updated to `opened`.
- When the user decides that the document is not relevant, and scrolls further, the status is updated to `skipped`.
- Sometimes if user changes the decision a `skipped` document can become `opened`.

#### Available transitions
- `missed` -> `presented`
- `presented` -> `skipped`
- `presented` -> `opened`
- `skipped` -> `opened`

```dart
enum DocumentStatus {
  skipped,
  presented,
  opened,
  missed,
}
```