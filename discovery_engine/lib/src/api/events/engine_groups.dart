import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/api/events/engine_events/feed_events.dart';
import 'package:xayn_discovery_engine/src/api/events/engine_events/system_events.dart';

part 'engine_groups.freezed.dart';
part 'engine_groups.g.dart';

abstract class EngineEvent {}

@freezed
class EngineEventGroups with _$EngineEventGroups {
  const factory EngineEventGroups.feed({
    required FeedEngineEvent event,
  }) = FeedClientGroup;

  const factory EngineEventGroups.system({
    required SystemEngineEvent event,
  }) = SystemClientGroup;

  factory EngineEventGroups.fromJson(Map<String, dynamic> json) =>
      _$EngineEventGroupsFromJson(json);
}
