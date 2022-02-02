# UniqueId and other identifiers

`UniqueId` represent base for unique identifiers for other models like "search" or `Document`.

```dart
abstract class UniqueId {
  final Uint8List value;
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
  final NewsResource resource;

  // These 2 fields can be private cause the UI 
  // only needs to know derived state:
  //  - if document is relevant or irrelevant
  //  - if document was opened
  final DocumentFeedback _feedback;
  final DocumentStatus _status;

  bool get isRelevant => _feedback => DocumentFeedback.positive;
  bool get isNotRelevant => _feedback == DocumentFeedback.negative;
  bool get wasOpened => _status == DocumentStatus.opened;

  // These 2 fields below will be used to sort documents
  // based on current personalisation state
  final int nonPersonalizedRank;
  final int personalizedRank;

  int get currentRank(bool isPersonalisationOn) => isPersonalisationOn
    ? personalizedRank 
    : nonPersonalizedRank;

  Document._({
    required this.documentId,
    required this.queryId,
    required this.resource,
    required this.nonPersonalizedRank,
    required this.personalizedRank,
    DocumentFeedback feedback = DocumentFeedback.neutral,
    DocumentStatus status = DocumentStatus.missed,
  }) : _feedback = feedback,
       _status = status;
}
```

## Attributes of a Document

### Document feedback

Indicates if the user "liked" or "disliked" the document.

```dart
enum DocumentFeedback {
  positive,
  neutral,
  negative,
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
  final Uri url;
  final Uri displayUrl;
}

class ImageResource extends WebResource {
  // additionally adds
  final Uri imageUrl;
  final int imageWidth;
  final int imageHeight;
  final Uri thumbnailUrl;
  final int thumbnailWidth;
  final int thumbnailHeight;
}

class NewsResource extends WebResource {
  // additionally adds
  final Uri thumbnailUrl;
  final int thumbnailWidth;
  final int thumbnailHeight;
  final String provider;
  final Set<String> topics;
  final DateTime datePublished;
}

class VideoResource extends WebResource {
  // additionally adds
  final Uri thumbnailUrl;
  final int thumbnailWidth;
  final int thumbnailHeight;
  final DateTime datePublished;
  final String publisher;
  final Uri videoUrl;
  final Uri motionThumbnailUrl;
  final int duration;
}

```
