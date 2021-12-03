import 'package:freezed_annotation/freezed_annotation.dart';

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
  }) = _WebResource;

  factory WebResource.fromJson(Map<String, Object?> json) =>
      _$WebResourceFromJson(json);
}
