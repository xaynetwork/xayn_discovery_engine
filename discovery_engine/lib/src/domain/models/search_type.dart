import 'package:freezed_annotation/freezed_annotation.dart';

/// Type of search documents that the client can request from the discovery engine.
enum SearchType {
  @JsonValue(0)
  web,
  @JsonValue(1)
  image,
  @JsonValue(2)
  video,
  @JsonValue(3)
  news,
}
