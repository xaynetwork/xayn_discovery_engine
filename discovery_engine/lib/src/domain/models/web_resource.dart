import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:xayn_discovery_engine/src/domain/models/web_resource_provider.dart'
    show WebResourceProvider, $WebResourceProviderCopyWith;

part 'web_resource.freezed.dart';
part 'web_resource.g.dart';

/// [WebResource] class is used to represent different kinds of resources
/// like web, image, video, news, etc. that are delivered by an external
/// content API, which might serve search results, news, or other types.
@freezed
class WebResource with _$WebResource {
  const factory WebResource({
    required String title,
    required String snippet,
    required Uri url,
    required Uri displayUrl,
    required DateTime datePublished,
    WebResourceProvider? provider,
  }) = _WebResource;

  factory WebResource.fromJson(Map<String, Object?> json) =>
      _$WebResourceFromJson(json);
}
