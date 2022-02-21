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

import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:hive/hive.dart'
    show HiveType, HiveField, TypeAdapter, BinaryReader, BinaryWriter;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show newsResourceTypeId;

part 'news_resource.freezed.dart';
part 'news_resource.g.dart';

/// [NewsResource] class is used to represent news that are
/// delivered by an external content API.
@freezed
class NewsResource with _$NewsResource {
  @HiveType(typeId: newsResourceTypeId)
  const factory NewsResource({
    @HiveField(0) required String title,
    @HiveField(1) required String snippet,
    @HiveField(2) required Uri url,
    @HiveField(3) required Uri sourceUrl,
    @HiveField(4) required Uri? image,
    @HiveField(5) required DateTime datePublished,
    @HiveField(6) required int rank,
    @HiveField(7) required double? score,
    @HiveField(8) required String country,
    @HiveField(9) required String language,
    @HiveField(10) required String topic,
  }) = _NewsResource;

  factory NewsResource.fromJson(Map<String, Object?> json) =>
      _$NewsResourceFromJson(json);
}
