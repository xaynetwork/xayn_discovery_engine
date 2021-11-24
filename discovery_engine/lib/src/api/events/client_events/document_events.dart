import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/api/events/client_groups.dart'
    show ClientEvent;
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';

part 'document_events.freezed.dart';
part 'document_events.g.dart';

@freezed
class DocumentClientEvent with _$DocumentClientEvent implements ClientEvent {
  /// Event created when the `DocumentStatus` changed:
  /// - when the document was presented to the user the status changes
  /// from `missed` to `presented`.
  /// - when the document was presented but then was scrolled out of the screen
  /// the status changes from `presented` to `skipped`. It means the user saw
  /// the document, but it wasn't relevant.
  /// - when the document was opened the status changes from `presented` or
  /// `skipped` to `opened`. It means the user was interested enough in
  /// the document to open it.
  const factory DocumentClientEvent.documentStatusChanged(
    DocumentId documentId,
    DocumentStatus status,
  ) = DocumentStatusChanged;

  /// Event created when the document was closed, either by going back to
  /// documents list or by navigating further to a link contained by the document.
  /// It helps to calculate how much time user spent reviewing the document.
  ///
  /// For cases when the user will open and close the same document multiple
  /// times (for the same search), the engine should store and use only
  /// the maximum time spent by the user on a document.
  const factory DocumentClientEvent.documentClosed(DocumentId documentId) =
      DocumentClosed;

  /// Event created when the user swipes the [Document] card or clicks a button
  /// to indicate that the document is `positive`, `negative` or `neutral`.
  const factory DocumentClientEvent.documentFeedbackChanged(
    DocumentId documentId,
    DocumentFeedback feedback,
  ) = DocumentFeedbackChanged;

  /// Converts json Map to [DocumentClientEvent].
  factory DocumentClientEvent.fromJson(Map<String, dynamic> json) =>
      _$DocumentClientEventFromJson(json);
}
