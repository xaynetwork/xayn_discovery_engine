# Document model

`Document` is the model exposed to the Xayn app for representing items in the discovery feed or in the search result list.

```dart
class Document {
  final UniqueId id;

  /// TODO: Do we need to have a relation to the Query?
  /// What about a Document in discovery feed?
  final UniqueId queryId;
  final WebResource webResource;
  final DocumentSentiment sentiment;

  Document({
    required this.id,
    required this.queryId,
    required this.webResource,
    DocumentSentiment sentiment = DocumentSentiment.neutral,
  }) : sentiment = sentiment;
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