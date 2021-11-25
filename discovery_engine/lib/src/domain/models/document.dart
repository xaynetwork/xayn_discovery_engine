import 'package:hive/hive.dart'
    show HiveType, HiveField, TypeAdapter, BinaryReader, BinaryWriter;
import 'package:json_annotation/json_annotation.dart' show JsonValue;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/models/web_resource.dart'
    show WebResource;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show documentTypeId, documentFeedbackTypeId, documentStatusTypeId;

part 'document.g.dart';

/// [Document] is representing items in the discovery feed
/// or in the search result list.
@HiveType(typeId: documentTypeId)
class Document {
  @HiveField(0)
  final DocumentId documentId;
  @HiveField(1)
  final WebResource webResource;
  @HiveField(2)
  final DocumentFeedback _feedback;
  @HiveField(3)
  final DocumentStatus _status;
  @HiveField(4)
  final int nonPersonalizedRank;
  @HiveField(5)
  final int personalizedRank;
  @HiveField(6)
  final bool _isActive;

  bool get isRelevant => _feedback == DocumentFeedback.positive;
  bool get isNotRelevant => _feedback == DocumentFeedback.negative;
  bool get isNeutral => _feedback == DocumentFeedback.neutral;
  bool get wasOpened => _status == DocumentStatus.opened;
  bool get isActive => _isActive;

  int currentRank(bool isPersonalisationOn) =>
      isPersonalisationOn ? personalizedRank : nonPersonalizedRank;

  Document({
    required this.webResource,
    required this.nonPersonalizedRank,
    required this.personalizedRank,
  })  : documentId = DocumentId(),
        _feedback = DocumentFeedback.neutral,
        _status = DocumentStatus.missed,
        _isActive = true;
}

/// [DocumentStatus] indicates what the document status is in the context of
/// other documents.
///   - Every document starts with `missed`, which means the user didn't have
/// the chance to see it.
///   - When a document is displayed to the user its status is updated to
/// `presented`.
///   - When the user decides to read it, the status is updated to `opened`.
///   - When the user decides that the document is not relevant, and scrolls
/// further, the status is updated to `skipped`.
///   - Sometimes if the user changes the decision a `skipped` document can
/// become `opened`.
@HiveType(typeId: documentStatusTypeId)
enum DocumentStatus {
  @JsonValue(0)
  @HiveField(0)
  skipped,
  @JsonValue(1)
  @HiveField(1)
  presented,
  @JsonValue(2)
  @HiveField(2)
  opened,
  @JsonValue(3)
  @HiveField(3)
  missed,
}

/// [DocumentFeedback] indicates user's "sentiment" towards the document,
/// essentially if the user "liked" or "disliked" the document.
@HiveType(typeId: documentFeedbackTypeId)
enum DocumentFeedback {
  @JsonValue(0)
  @HiveField(0)
  neutral,
  @JsonValue(1)
  @HiveField(1)
  positive,
  @JsonValue(2)
  @HiveField(2)
  negative,
}
