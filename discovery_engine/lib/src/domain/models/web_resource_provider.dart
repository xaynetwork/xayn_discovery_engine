// Copyright 2022 Xayn AG
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
    show webResourceProviderTypeId;

part 'web_resource_provider.freezed.dart';
part 'web_resource_provider.g.dart';

/// The [WebResourceProvider] class represents the provider of a `WebResource`.
/// [name] represents the provider's legal name
/// [thumbnail] is `Uri` which contains a link to the thumbnail-sized logo for the provider.
@freezed
class WebResourceProvider with _$WebResourceProvider {
  @HiveType(typeId: webResourceProviderTypeId)
  const factory WebResourceProvider({
    @HiveField(0) required String name,
    @HiveField(1) required Uri? thumbnail,
  }) = _WebResourceProvider;

  factory WebResourceProvider.fromJson(Map<String, Object?> json) =>
      _$WebResourceProviderFromJson(json);
}
