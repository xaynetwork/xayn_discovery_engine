import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/api/events/client_events/document_events.dart';
import 'package:xayn_discovery_engine/src/api/events/client_events/feed_events.dart';
import 'package:xayn_discovery_engine/src/api/events/client_events/system_events.dart';

part 'client_groups.freezed.dart';
part 'client_groups.g.dart';

abstract class ClientEvent {}

@freezed
class ClientEventGroups with _$ClientEventGroups {
  const factory ClientEventGroups.document({
    required DocumentClientEvent event,
  }) = DocumentClientGroup;

  const factory ClientEventGroups.feed({
    required FeedClientEvent event,
  }) = FeedClientGroup;

  const factory ClientEventGroups.system({
    required SystemClientEvent event,
  }) = SystemClientGroup;

  factory ClientEventGroups.fromJson(Map<String, dynamic> json) =>
      _$ClientEventGroupsFromJson(json);
}
