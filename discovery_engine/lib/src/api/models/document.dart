import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show DocumentFeedback, DocumentStatus;
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
