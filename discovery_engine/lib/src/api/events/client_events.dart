import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/search_type.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';

part 'client_events.freezed.dart';
part 'client_events.g.dart';

@freezed
class ClientEvent with _$ClientEvent {
  /// Event created upon every app startup, with some data needed
  /// for the engine to work, like personalisation and feed market
  /// (for performing background queries).
  const factory ClientEvent.init(
    Configuration configuration, {
    @Default(true) bool isPersonalisationOn,
  }) = Init;

  /// Event created when the app decides to reset the AI (start fresh).
  const factory ClientEvent.resetEngine() = ResetEngine;

  /// Event created when the user toggles the AI on/off.
  ///
  /// When the personalisation is OFF:
  ///  - we are still reranking all the incoming results, but we don't use
  /// personal data to do it
  ///  - we are preventing storing queries and documents in the history,
  /// and sending/processing document-related events (likes, dislikes, etc.)
  ///
  /// Every document gets a rank from the reranker only once. When we toggle
  /// we switch between the API rank and Engine rank.
  const factory ClientEvent.personalizationChanged(bool isOn) =
      PersonalizationChanged;

  /// Event created when the user changes market for the feed ie.
  /// in global settings or changes some parameters for search,
  /// like market or count (nb of items per page).
  const factory ClientEvent.configurationChanged({
    String? feedMarket,
    String? searchMarket,
    int? maxItemsPerSearchBatch,
    int? maxItemsPerFeedBatch,
  }) = ConfigurationChanged;

  /// Event created when opening up discovery screen (upon initial start
  /// of the app or when we are returning to previously displayed
  /// discovery feed). When restoring previous feed it returns all the documents,
  /// that were still accessible to the user, so they weren't closed by
  /// the [FeedDocumentsClosed] event.
  const factory ClientEvent.feedRequested() = FeedRequested;

  /// Event created when the app wants to requests new content
  /// for the discovery feed:
  ///  - when reaching the end of the current list of items
  ///  - in response to `NewFeedAvailable` event, or after deliberate user action
  /// like pressing the button to fetch new items
  ///  - on some time trigger
  ///  - as a follow up when changing the news market
  const factory ClientEvent.newFeedRequested() = NewFeedRequested;

  /// Event created when the client makes `Documents` in the feed not accessible
  /// to the user anymore. The engine registers those documents as immutable,
  /// so they can't be changed anymore by the client.
  const factory ClientEvent.feedDocumentsClosed(Set<DocumentId> documentIds) =
      FeedDocumentsClosed;

  /// Event created when the user triggers a search query:
  ///  - by typing the search term aka. "real-time search"
  ///  - by deep search on a document
  ///  - by selecting item provided by autosuggestion
  ///  - by selecting item from history of past searches
  ///  - by changing the search market
  ///  - by changing the type of search
  const factory ClientEvent.searchRequested({
    required String term,
    // Search types => web, image, video, news, etc.
    required List<SearchType> types,
  }) = SearchRequested;

  /// Event created when the user triggers a request for next batch
  /// of the current search, usually by scrolling to the end of the results list.
  const factory ClientEvent.nextSearchBatchRequested(SearchId searchId) =
      NextSearchBatchRequested;

  /// Event created when we want to restore a previous search state. The engine
  /// will respond with all related `Documents` fetched and reranked
  /// on the previous app "run".
  ///
  /// The `searchId` needs to remain "stable" from the app point of view. When
  /// restoring previous search the engine should give back all the documents
  /// that are related to that `searchId`.
  ///
  /// When asking for a next page of results for that "old" query, but during
  /// a "new" session, the engine needs to send back documents that are related
  /// to the same "old" `searchId`.
  const factory ClientEvent.searchRestoreRequested(SearchId searchId) =
      SearchRestoreRequested;

  /// Event created when the client wants to know which searches
  /// the discovery engine can restore.
  const factory ClientEvent.activeSearchesRequested() = ActiveSearchesRequested;

  /// Event created when a search and related Documents can't be accessed again
  /// by the user from the UI. Usualy it happens when the users closes a tab/tabs,
  /// so all searches within these tabs are also closed.
  const factory ClientEvent.searchesClosed(Set<SearchId> searchIds) =
      SearchesClosed;

  /// Event created when the `DocumentStatus` changed:
  /// - when the document was presented to the user the status changes
  /// from `missed` to `presented`.
  /// - when the document was presented but then was scrolled out of the screen
  /// the status changes from `presented` to `skipped`. It means the user saw
  /// the document, but it wasn't relevant.
  /// - when the document was opened the status changes from `presented` or
  /// `skipped` to `opened`. It means the user was interested enough in
  /// the document to open it.
  const factory ClientEvent.documentStatusChanged(
    DocumentId documentId,
    DocumentStatus status,
  ) = DocumentStatusChanged;

  /// Same as [DocumentStatusChanged] with `DocumentStatus.opened` but for pages
  /// in the webview that didn't originate from a list of documents:
  /// - opened an external url, from a different app
  /// - opened as a direct url, by typing it in the search field
  /// - navigated to inside of the webview, after clicking on a link
  const factory ClientEvent.urlOpened({
    required String url,
    required String title,
    required String snippet,
  }) = UrlOpened;

  /// Event created when the document was closed, either by going back to
  /// documents list or by navigating further to a link contained by the document.
  /// It helps to calculate how much time user spent reviewing the document.
  ///
  /// For cases when the user will open and close the same document multiple
  /// times (for the same search), the engine should store and use only
  /// the maximum time spent by the user on a document.
  const factory ClientEvent.documentClosed(DocumentId documentId) =
      DocumentClosed;

  /// Event created when the user swipes the [Document] card or clicks a button
  /// to indicate that the document is `positive`, `negative` or `neutral`.
  const factory ClientEvent.documentFeedbackChanged(
    DocumentId documentId,
    DocumentFeedback feedback,
  ) = DocumentFeedbackChanged;

  /// Event created when the user bookmarks a document. Engine internally could
  /// treat it as `like`.
  const factory ClientEvent.bookmarkCreated(DocumentId documentId) =
      BookmarkCreated;

  /// Event created when the user removed single or multiple bookmarks. Engine
  /// internally could treat it as `neutral`.
  const factory ClientEvent.bookmarksRemoved(Set<DocumentId> documentIds) =
      BookmarksRemoved;

  /// Event created when the user dismisses categories/topics when doing
  /// a "negative" swipe, ie. on item in the news feed.
  const factory ClientEvent.contentCategoriesDismissed(
    DocumentId documentId,
    Set<String> categories,
  ) = ContentCategoriesDismissed;

  /// Event created when the user removes "ban" from previously dismisses feed
  /// categories/topics.
  const factory ClientEvent.contentCategoriesAccepted(Set<String> categories) =
      ContentCategoriesAccepted;

  /// Converts json Map to [ClientEvent].
  factory ClientEvent.fromJson(Map<String, dynamic> json) =>
      _$ClientEventFromJson(json);
}
