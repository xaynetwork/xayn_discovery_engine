import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/api/events/client_groups.dart'
    show ClientEvent;
import 'package:xayn_discovery_engine/src/domain/models/search_type.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';

part 'search_events.freezed.dart';
part 'search_events.g.dart';

@freezed
class SearchClientEvent with _$SearchClientEvent implements ClientEvent {
  /// Event created when the user triggers a search query:
  ///  - by typing the search term aka. "real-time search"
  ///  - by deep search on a document
  ///  - by selecting item provided by autosuggestion
  ///  - by selecting item from history of past searches
  ///  - by changing the search market
  ///  - by changing the type of search
  const factory SearchClientEvent.searchRequested({
    required String term,
    // Search types => web, image, video, news, etc.
    required List<SearchType> types,
  }) = SearchRequested;

  /// Event created when the user triggers a request for next batch
  /// of the current search, usually by scrolling to the end of the results list.
  const factory SearchClientEvent.nextSearchBatchRequested(SearchId searchId) =
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
  const factory SearchClientEvent.searchRestoreRequested(SearchId searchId) =
      SearchRestoreRequested;

  /// Event created when the client wants to know which searches
  /// the discovery engine can restore.
  const factory SearchClientEvent.activeSearchesRequested() =
      ActiveSearchesRequested;

  /// Event created when a search and related Documents can't be accessed again
  /// by the user from the UI. Usualy it happens when the users closes a tab/tabs,
  /// so all searches within these tabs are also closed.
  const factory SearchClientEvent.searchesClosed(Set<SearchId> searchIds) =
      SearchesClosed;

  /// Converts json Map to [SearchClientEvent].
  factory SearchClientEvent.fromJson(Map<String, dynamic> json) =>
      _$SearchClientEventFromJson(json);
}
