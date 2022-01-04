import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:hive/hive.dart' show HiveType, HiveField;

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
