import 'package:xayn_discovery_engine/src/api/events/base_events.dart'
    show ClientEvent, EngineEvent;
import 'package:xayn_discovery_engine/src/api/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show DocumentFeedback, DocumentStatus;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Event created when the `DocumentStatus` changed:
/// - when the document was presented to the user the status changes
/// from `missed` to `presented`.
/// - when the document was presented but then was scrolled out of the screen
/// the status changes from `presented` to `skipped`. It means the user saw
/// the document, but it wasn't relevant.
/// - when the document was opened the status changes from `presented` or
/// `skipped` to `opened`. It means the user was interested enough in
/// the document to open it.
class DocumentStatusChanged extends ClientEvent {
  final DocumentId documentId;
  final DocumentStatus status;

  const DocumentStatusChanged(this.documentId, this.status);
}

/// Same as [DocumentStatusChanged] with `DocumentStatus.opened` but for pages
/// in the webview that didn't originate from a list of documents:
/// - opened an external url, from a different app
/// - opened as a direct url, by typing it in the search field
/// - navigated to inside of the webview, after clicking on a link
class UrlOpened extends ClientEvent {
  final String url;
  final String title;
  final String snippet;

  const UrlOpened(this.url, this.title, this.snippet);
}

/// Event created as a response to [UrlOpened] event which
/// contains [DocumentId] to be used with other "document" events,
/// like [DocumentClosed], [DocumentFeedbackChanged], etc.
class DocumentFromUrlCreated extends EngineEvent {
  final DocumentId documentId;

  const DocumentFromUrlCreated(this.documentId);
}

/// Event created when the document was closed, either by going back to
/// documents list or by navigating further to a link contained by the document.
/// It helps to calculate how much time user spent reviewing the document.
///
/// For cases when the user will open and close the same document multiple
/// times (for the same search), the engine should store and use only
/// the maximum time spent by the user on a document.
class DocumentClosed extends ClientEvent {
  final DocumentId documentId;

  const DocumentClosed(this.documentId);
}

/// Event created when the user swipes the [Document] card or clicks a button
/// to indicate that the document is `positive`, `negative` or `neutral`.
class DocumentFeedbackChanged extends ClientEvent {
  final DocumentId documentId;
  final DocumentFeedback feedback;

  const DocumentFeedbackChanged(this.documentId, this.feedback);
}

/// Event created when the user bookmarks a document. Engine internally could
/// treat it as `like`.
class BookmarkCreated extends ClientEvent {
  final DocumentId documentId;

  const BookmarkCreated(this.documentId);
}

/// Event created when the user removed single or multiple bookmarks. Engine
/// internally could treat it as `neutral`.
class BookmarksRemoved extends ClientEvent {
  final Set<DocumentId> documentIds;

  const BookmarksRemoved(this.documentIds);
}
