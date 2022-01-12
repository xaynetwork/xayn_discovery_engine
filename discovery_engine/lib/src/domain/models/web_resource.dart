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
import 'package:xayn_discovery_engine/src/domain/models/web_resource_provider.dart';
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show webResourceTypeId;

part 'web_resource.freezed.dart';
part 'web_resource.g.dart';

/// [WebResource] class is used to represent different kinds of resources
/// like web, image, video, news, etc. that are delivered by an external
/// content API, which might serve search results, news, or other types.
@freezed
class WebResource with _$WebResource {
  @HiveType(typeId: webResourceTypeId)
  const factory WebResource({
    @HiveField(0) required String title,
    @HiveField(1) required String snippet,
    @HiveField(2) required Uri url,
    @HiveField(3) required Uri displayUrl,
    @HiveField(4) required DateTime datePublished,
    @HiveField(5) WebResourceProvider? provider,
  }) = _WebResource;

  factory WebResource.fromJson(Map<String, Object?> json) =>
      _$WebResourceFromJson(json);
}
