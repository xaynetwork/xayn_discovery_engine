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
import 'package:xayn_discovery_engine/src/api/models/document.dart' as api;
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResource;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show documentTypeId, userReactionTypeId;

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
  final NewsResource resource;
  @HiveField(3)
  UserReaction feedback;
  @HiveField(4)
  final int personalizedRank;
  @HiveField(5)
  bool isActive;
  @HiveField(6)
  DateTime timestamp;

  bool get isRelevant => feedback == UserReaction.positive;
  bool get isNotRelevant => feedback == UserReaction.negative;
  bool get isNeutral => feedback == UserReaction.neutral;

  Document({
    required this.stackId,
    required this.resource,
    required this.personalizedRank,
    this.feedback = UserReaction.neutral,
    this.isActive = true,
  })  : documentId = DocumentId(),
        timestamp = DateTime.now().toUtc();

  api.Document toApiDocument() => api.Document(
        documentId: documentId,
        resource: resource,
        feedback: feedback,
        nonPersonalizedRank: personalizedRank, // TODO remove?
        personalizedRank: personalizedRank,
        isActive: isActive,
      );
}

/// [UserReaction] indicates user's "sentiment" towards the document,
/// essentially if the user "liked" or "disliked" the document.
@HiveType(typeId: userReactionTypeId)
enum UserReaction {
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
