import 'web_resource.dart';
import 'unique_id.dart';

/// [Document] is representing items in the discovery feed
/// or in the search result list.
class Document {
  final DocumentId documentId;
  final WebResource webResource;
  final DocumentFeedback _feedback;
  final DocumentStatus _status;
  final int nonPersonalizedRank;
  final int personalizedRank;

  bool get isRelevant => _feedback == DocumentFeedback.positive;
  bool get isNotRelevant => _feedback == DocumentFeedback.negative;
  bool get isNeutral => _feedback == DocumentFeedback.neutral;
  bool get wasOpened => _status == DocumentStatus.opened;

  int currentRank(bool isPersonalisationOn) =>
      isPersonalisationOn ? personalizedRank : nonPersonalizedRank;

  Document._({
    required this.webResource,
    required this.nonPersonalizedRank,
    required this.personalizedRank,
  })  : documentId = DocumentId(),
        _feedback = DocumentFeedback.neutral,
        _status = DocumentStatus.missed;
}

/// The status of the document is not exposed to the Xayn app, because it has no usage in the UI (apart from `opened` status which could be used to indicate that the document was "visited", which is visually represented by decreased opacity of the document list item).
///
/// It indicates what is the document status in context of other documents.
///   - Every document starts with `missed`, which means the user didn't have the chance to see it.
///   - When a document is displayed to the user its status is updated to `presented`.
///   - When the user decides to read it, the status is updated to `opened`.
///   - When the user decides that the document is not relevant, and scrolls further, the status is updated to `skipped`.
///   - Sometimes if user changes the decision a `skipped` document can become `opened`.
enum DocumentStatus {
  skipped,
  presented,
  opened,
  missed,
}

extension _IntToDocumentStatusExt on int {
  DocumentStatus toDocumentStatus() {
    switch (this) {
      case 0:
        return DocumentStatus.skipped;
      case 1:
        return DocumentStatus.presented;
      case 2:
        return DocumentStatus.opened;
      case 3:
      default:
        return DocumentStatus.missed;
    }
  }
}

/// [DocumentFeedback] indicates user's "sentiment" towards the document.
///
/// if the user "liked" or "disliked" the document.
enum DocumentFeedback {
  neutral,
  positive,
  negative,
}

extension _IntToDocumentFeedbackExt on int {
  DocumentFeedback toDocumentFeedback() {
    switch (this) {
      case 1:
        return DocumentFeedback.positive;
      case 2:
        return DocumentFeedback.negative;
      case 0:
      default:
        return DocumentFeedback.neutral;
    }
  }
}
