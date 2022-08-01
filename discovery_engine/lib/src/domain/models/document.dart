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
import 'package:json_annotation/json_annotation.dart'
    show $enumDecode, JsonEnum, JsonValue;
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResource;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show documentTypeId, userReactionTypeId;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustUserReaction;

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
  UserReaction userReaction;
  @HiveField(5)
  bool isActive;

  @Deprecated('only available without `storage` enabled')
  @HiveField(4)
  final int batchIndex;
  @Deprecated('only available without `storage` enabled')
  @HiveField(6)
  DateTime timestamp;

  /// Indicates if this [Document] was returned in response to active search.
  //soft deprecated, will be replaced with getter once we migrated to `storage`
  @HiveField(7)
  bool isSearched;

  bool get isRelevant => userReaction == UserReaction.positive;
  bool get isNotRelevant => userReaction == UserReaction.negative;
  bool get isNeutral => userReaction == UserReaction.neutral;

  /// Returns [NewsResource] snippet, or title if snippet is an empty String.
  String get snippet =>
      resource.snippet.isNotEmpty ? resource.snippet : resource.title;

  Document({
    required this.stackId,
    required this.resource,
    // ignore: deprecated_consistency
    required this.batchIndex,
    required this.documentId,
    this.userReaction = UserReaction.neutral,
    this.isActive = true,
    // ignore: deprecated_consistency
    this.isSearched = false,
    // ignore: deprecated_member_use_from_same_package
  }) : timestamp = DateTime.now().toUtc();
}

/// [UserReaction] indicates user's "sentiment" towards the document,
/// essentially if the user "liked" or "disliked" the document.
@HiveType(typeId: userReactionTypeId)
@JsonEnum(alwaysCreate: true)
enum UserReaction {
  @JsonValue(RustUserReaction.Neutral)
  @HiveField(RustUserReaction.Neutral)
  neutral,
  @JsonValue(RustUserReaction.Positive)
  @HiveField(RustUserReaction.Positive)
  positive,
  @JsonValue(RustUserReaction.Negative)
  @HiveField(RustUserReaction.Negative)
  negative,
}

extension UserReactionIntConversion on UserReaction {
  int toIntRepr() => _$UserReactionEnumMap[this]!;
  static UserReaction fromIntRepr(int intRepr) =>
      $enumDecode(_$UserReactionEnumMap, intRepr);
}
