import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/models/web_resource.dart'
    show WebResource;

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

/// [DocumentStatus] indicates what is the document status in context of other
/// documents.
///   - Every document starts with `missed`, which means the user didn't have
/// the chance to see it.
///   - When a document is displayed to the user its status is updated to
/// `presented`.
///   - When the user decides to read it, the status is updated to `opened`.
///   - When the user decides that the document is not relevant, and scrolls
/// further, the status is updated to `skipped`.
///   - Sometimes if user changes the decision a `skipped` document can become
/// `opened`.
enum DocumentStatus {
  skipped,
  presented,
  opened,
  missed,
}

/// [DocumentFeedback] indicates user's "sentiment" towards the document,
/// essentially if the user "liked" or "disliked" the document.
enum DocumentFeedback {
  neutral,
  positive,
  negative,
}
