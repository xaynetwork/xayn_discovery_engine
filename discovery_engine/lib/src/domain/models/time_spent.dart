import 'dart:typed_data';

import 'package:freezed_annotation/freezed_annotation.dart' show freezed;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

part 'time_spent.freezed.dart';

/// TimeSpent event with metadata as passed to the engine.
@freezed
class TimeSpent with _$TimeSpent {
  const factory TimeSpent({
    required DocumentId id,
    required Float32List smbertEmbedding,
    required Duration time,
    required UserReaction reaction,
  }) = _TimeSpent;
}
