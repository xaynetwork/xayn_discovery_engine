import 'package:hive/hive.dart'
    show HiveType, HiveField, TypeAdapter, BinaryReader, BinaryWriter;
import 'package:json_annotation/json_annotation.dart' show JsonValue;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/models/web_resource.dart'
    show WebResource;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show documentTypeId, documentFeedbackTypeId;

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
  DocumentFeedback feedback;
  @HiveField(3)
  final int personalizedRank;
  @HiveField(4)
  bool isActive;

  bool get isRelevant => feedback == DocumentFeedback.positive;
  bool get isNotRelevant => feedback == DocumentFeedback.negative;
  bool get isNeutral => feedback == DocumentFeedback.neutral;

  Document({
    required this.webResource,
    required this.personalizedRank,
    this.feedback = DocumentFeedback.neutral,
    this.isActive = true,
  }) : documentId = DocumentId();
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
