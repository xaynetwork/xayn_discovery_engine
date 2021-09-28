# Identifiers

```dart
abstract class UniqueId {
  /// this could be also `int` or other type, but needs to be serializable
  final String value;
}

/// Identifier for searches
class SearchId extends UniqueId {}

/// Identifier for documents
class DocumentId extends UniqueId {}
```

# Document model

`Document` is the model exposed to the Xayn app for representing items in the discovery feed or in the search result list.

```dart
class Document {
  /// Unique identifier of a document
  final DocumentId documentId;

  /// Contains all data from search API that are needed for the UI.
  /// It's a base class for web, news, video, image resources.
  final WebResource webResource;

  // These 2 fields can be private cause the UI 
  // only needs to know derived state:
  //  - if document is relevant or irrelevant
  //  - if document was opened
  final DocumentFeedback _feedback;
  final DocumentStatus _status;

  bool get isRelevant => _feedback => DocumentFeedback.liked;
  bool get isNotRelevant => _feedback == DocumentFeedback.disliked;
  bool get wasOpened => _status == DocumentStatus.opened;

  // These 2 fields below will be used to sort documents
  // based on current personalisation state
  final int apiRank;
  final int engineRank;

  int get rank(bool isPersonalisationOn) => isPersonalisationOn
    ? _engineRank 
    : _apiRank;

  Document._({
    required this.documentId,
    required this.queryId,
    required this.webResource,
    required this.apiRank,
    required this.engineRank,
    DocumentFeedback feedback = DocumentFeedback.neutral,
    DocumentStatus status = DocumentStatus.missed,
  }) : _feedback = feedback,
       _status = status;
}
```

## Attributes of a Document

### Document feedback

Indicates if the user `liked` or `disliked` the document.

```dart
enum DocumentFeedback {
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

## WebResource

Class used to represent different kind of resources like web, image, video, news, etc. 

```dart
class WebResource {
  final String title;
  final String snippet;
  final String url;
  final String displayUrl;
}

class ImageResource extends WebResource {
  // additionally adds
  final String imageUrl;
  final int imageWidth;
  final int imageHeight;
  final String thumbnailUrl;
  final int thumbnailWidth;
  final int thumbnailHeight;
}

class NewsResource extends WebResource {
  // additionally adds
  final String thumbnailUrl;
  final int thumbnailWidth;
  final int thumbnailHeight;
  final String provider;
  final Set<String> topics;
  final DateTime datePublished;
}

class VideoResource extends WebResource {
  // additionally adds
  final String thumbnailUrl;
  final int thumbnailWidth;
  final int thumbnailHeight;
  final DateTime datePublished;
  final String publisher;
  final String videoUrl;
  final String motionThumbnailUrl;
  final int duration;
}

```
