import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show DocumentFeedback;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/models/web_resource.dart'
    show WebResource, $WebResourceCopyWith;

part 'document.freezed.dart';
part 'document.g.dart';

/// [Document] is representing items in the discovery feed
/// or in the search result list.
@freezed
class Document with _$Document {
  const Document._();

  const factory Document({
    required DocumentId documentId,
    required WebResource webResource,
    required DocumentFeedback feedback,
    required int nonPersonalizedRank,
    required int personalizedRank,
    required bool isActive,
  }) = _Document;

  /// Converts json Map to [Document].
  factory Document.fromJson(Map<String, Object?> json) =>
      _$DocumentFromJson(json);
}
