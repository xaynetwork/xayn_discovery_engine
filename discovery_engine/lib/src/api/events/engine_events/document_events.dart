import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/api/events/engine_groups.dart'
    show EngineEvent;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';

part 'document_events.freezed.dart';
part 'document_events.g.dart';

@freezed
class DocumentEngineEvent with _$DocumentEngineEvent implements EngineEvent {
  /// Event created as a response to UrlOpened event which
  /// contains [DocumentId] to be used with other "document" events,
  /// like DocumentClosed, DocumentFeedbackChanged, etc.
  const factory DocumentEngineEvent.documentFromUrlCreated(
    DocumentId documentId,
  ) = DocumentFromUrlCreated;

  /// Converts json Map to [DocumentEngineEvent].
  factory DocumentEngineEvent.fromJson(Map<String, dynamic> json) =>
      _$DocumentEngineEventFromJson(json);
}
