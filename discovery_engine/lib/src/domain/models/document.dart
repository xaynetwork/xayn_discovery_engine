// Copyright 2021 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

import 'package:hive/hive.dart'
    show HiveType, HiveField, TypeAdapter, BinaryReader, BinaryWriter;
import 'package:json_annotation/json_annotation.dart' show JsonValue;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
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
  final StackId stackId;
  @HiveField(2)
  final WebResource webResource;
  @HiveField(3)
  DocumentFeedback feedback;
  @HiveField(4)
  final int personalizedRank;
  @HiveField(5)
  bool isActive;
  @HiveField(6)
  DateTime timestamp;

  bool get isRelevant => feedback == DocumentFeedback.positive;
  bool get isNotRelevant => feedback == DocumentFeedback.negative;
  bool get isNeutral => feedback == DocumentFeedback.neutral;

  Document({
    required this.stackId,
    required this.webResource,
    required this.personalizedRank,
    this.feedback = DocumentFeedback.neutral,
    this.isActive = true,
  })  : documentId = DocumentId(),
        timestamp = DateTime.now().toUtc();
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
