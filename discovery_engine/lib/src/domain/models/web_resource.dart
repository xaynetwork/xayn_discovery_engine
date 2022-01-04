import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:hive/hive.dart'
    show HiveType, HiveField, TypeAdapter, BinaryReader, BinaryWriter;
import 'package:xayn_discovery_engine/src/domain/models/web_resource_provider.dart';
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show webResourceTypeId, uriTypeId;

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

class UriAdapter extends TypeAdapter<Uri> {
  @override
  final typeId = uriTypeId;

  @override
  Uri read(BinaryReader reader) {
    final uriStr = reader.readString();
    return Uri.parse(uriStr);
  }

  @override
  void write(BinaryWriter writer, Uri obj) {
    writer.writeString(obj.toString());
  }
}
