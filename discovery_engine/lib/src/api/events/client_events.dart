import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, DocumentStatus, DocumentFeedback;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

part 'client_events.freezed.dart';
part 'client_events.g.dart';

/// Abstract class implemented by events like [DocumentStatusChanged],
/// [DocumentFeedbackChanged] or [DocumentClosed].
///
/// Used to group events related to [Document] changes.
abstract class DocumentClientEvent {}

/// Abstract class implemented by events like [FeedRequested],
/// [NextFeedBatchRequested] or [FeedDocumentsClosed].
///
/// Used to group discovery feed related events.
abstract class FeedClientEvent {}

/// Abstract class implemented by events like [Init], [ResetEngine] or
/// [ConfigurationChanged].
///
/// Used to group generic system events.
abstract class SystemClientEvent {}

@freezed
class ClientEvent with _$ClientEvent {
  /// Event created upon every app startup, with some data needed
  /// for the engine to work, like personalisation and feed market
  /// (for performing background queries).
  @Implements<SystemClientEvent>()
  const factory ClientEvent.init(
    Configuration configuration,
  ) = Init;

  /// Event created when the app decides to reset the AI (start fresh).
  @Implements<SystemClientEvent>()
  const factory ClientEvent.resetEngine() = ResetEngine;

  /// Event created when the user changes market or count (nb of items per page)
  /// for the feed ie. in global settings.
  @Implements<SystemClientEvent>()
  const factory ClientEvent.configurationChanged({
    String? feedMarket,
    int? maxItemsPerFeedBatch,
  }) = ConfigurationChanged;

  /// Event created when opening up discovery screen (upon initial start
  /// of the app or when we are returning to the previously displayed
  /// discovery feed). When restoring the previous feed it returns all the documents
  /// that were still accessible to the user, namely those that weren't closed in
  /// the [FeedDocumentsClosed] event.
  @Implements<FeedClientEvent>()
  const factory ClientEvent.feedRequested() = FeedRequested;

  /// Event created when the app wants to request new content
  /// for the discovery feed:
  ///  - when reaching the end of the current list of items
  ///  - in response to `NextFeedBatchAvailable` event, or after deliberate user action
  /// like pressing the button to fetch new items
  ///  - on some time trigger
  ///  - as a follow up when changing the configuration
  @Implements<FeedClientEvent>()
  const factory ClientEvent.nextFeedBatchRequested() = NextFeedBatchRequested;

  /// Event created when the client makes [Document]s in the feed not accessible
  /// to the user anymore. The engine registers those documents as immutable,
  /// so they can't be changed anymore by the client.
  @Implements<FeedClientEvent>()
  const factory ClientEvent.feedDocumentsClosed(
    Set<DocumentId> documentIds,
  ) = FeedDocumentsClosed;

  /// Event created when the `DocumentStatus` changed:
  /// - when the document was presented to the user the status changes
  /// from `missed` to `presented`.
  /// - when the document was presented but then was scrolled out of the screen
  /// the status changes from `presented` to `skipped`. It means the user saw
  /// the document, but it wasn't relevant.
  /// - when the document was opened the status changes from `presented` or
  /// `skipped` to `opened`. It means the user was interested enough in
  /// the document to open it.
  @Implements<DocumentClientEvent>()
  const factory ClientEvent.documentStatusChanged(
    DocumentId documentId,
    DocumentStatus status,
  ) = DocumentStatusChanged;

  /// Event created when the user swipes the [Document] card or clicks a button
  /// to indicate that the document is `positive`, `negative` or `neutral`.
  @Implements<DocumentClientEvent>()
  const factory ClientEvent.documentFeedbackChanged(
    DocumentId documentId,
    DocumentFeedback feedback,
  ) = DocumentFeedbackChanged;

  /// Event created when the document was closed, either by going back to
  /// documents list or by navigating further to a link contained by the document.
  /// It helps to calculate how much time user spent reviewing the document.
  ///
  /// For cases when the user will open and close the same document multiple
  /// times, the engine should store and use only the maximum time spent
  /// by the user on a document.
  @Implements<DocumentClientEvent>()
  const factory ClientEvent.documentClosed(DocumentId documentId) =
      DocumentClosed;

  /// Converts json Map to [ClientEvent].
  factory ClientEvent.fromJson(Map<String, Object?> json) =>
      _$ClientEventFromJson(json);
}
