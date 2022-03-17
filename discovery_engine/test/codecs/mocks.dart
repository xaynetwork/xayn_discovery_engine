// Copyright 2021 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

import 'package:xayn_discovery_engine/src/api/api.dart'
    show
        ActiveSearch,
        ClientEvent,
        ClientEventSucceeded,
        Configuration,
        ConfigurationChanged,
        Document,
        DocumentId,
        DocumentTimeSpent,
        DocumentViewMode,
        DocumentsUpdated,
        EngineEvent,
        EngineExceptionRaised,
        EngineExceptionReason,
        ExcludedSourceAdded,
        ExcludedSourceRemoved,
        ExcludedSourcesListRequestFailed,
        ExcludedSourcesListRequestSucceeded,
        ExcludedSourcesListRequested,
        FeedDocumentsClosed,
        FeedFailureReason,
        FeedMarkets,
        FetchingAssetsFinished,
        FetchingAssetsProgressed,
        FetchingAssetsStarted,
        Init,
        NextFeedBatchAvailable,
        NextFeedBatchRequestFailed,
        NextFeedBatchRequestSucceeded,
        NextFeedBatchRequested,
        NextSearchBatchRequestFailed,
        NextSearchBatchRequestSucceeded,
        NextSearchBatchRequested,
        RestoreFeedFailed,
        RestoreFeedRequested,
        RestoreFeedSucceeded,
        RestoreSearchFailed,
        RestoreSearchRequested,
        RestoreSearchSucceeded,
        SearchClosed,
        SearchFailureReason,
        SearchRequestFailed,
        SearchRequestSucceeded,
        SearchRequested,
        SearchTermRequestFailed,
        SearchTermRequestSucceeded,
        SearchTermRequested,
        UserReaction,
        UserReactionChanged;

class BadClientEvent implements ClientEvent {
  const BadClientEvent();

  @override
  TResult map<TResult extends Object?>({
    required TResult Function(Init value) init,
    required TResult Function(ConfigurationChanged value) configurationChanged,
    required TResult Function(RestoreFeedRequested value) restoreFeedRequested,
    required TResult Function(NextFeedBatchRequested value)
        nextFeedBatchRequested,
    required TResult Function(FeedDocumentsClosed value) feedDocumentsClosed,
    required TResult Function(ExcludedSourceAdded value) excludedSourceAdded,
    required TResult Function(ExcludedSourceRemoved value)
        excludedSourceRemoved,
    required TResult Function(ExcludedSourcesListRequested value)
        excludedSourcesListRequested,
    required TResult Function(DocumentTimeSpent value) documentTimeSpent,
    required TResult Function(UserReactionChanged value) userReactionChanged,
    required TResult Function(SearchRequested value) searchRequested,
    required TResult Function(NextSearchBatchRequested value)
        nextSearchBatchRequested,
    required TResult Function(RestoreSearchRequested value)
        restoreSearchRequested,
    required TResult Function(SearchClosed value) searchClosed,
    required TResult Function(SearchTermRequested value) searchTermRequested,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult? mapOrNull<TResult extends Object?>({
    TResult Function(Init value)? init,
    TResult Function(ConfigurationChanged value)? configurationChanged,
    TResult Function(RestoreFeedRequested value)? restoreFeedRequested,
    TResult Function(NextFeedBatchRequested value)? nextFeedBatchRequested,
    TResult Function(FeedDocumentsClosed value)? feedDocumentsClosed,
    TResult Function(ExcludedSourceAdded value)? excludedSourceAdded,
    TResult Function(ExcludedSourceRemoved value)? excludedSourceRemoved,
    TResult Function(ExcludedSourcesListRequested value)?
        excludedSourcesListRequested,
    TResult Function(DocumentTimeSpent value)? documentTimeSpent,
    TResult Function(UserReactionChanged value)? userReactionChanged,
    TResult Function(SearchRequested value)? searchRequested,
    TResult Function(NextSearchBatchRequested value)? nextSearchBatchRequested,
    TResult Function(RestoreSearchRequested value)? restoreSearchRequested,
    TResult Function(SearchClosed value)? searchClosed,
    TResult Function(SearchTermRequested value)? searchTermRequested,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult maybeMap<TResult extends Object?>({
    required TResult Function() orElse,
    TResult Function(Init value)? init,
    TResult Function(ConfigurationChanged value)? configurationChanged,
    TResult Function(RestoreFeedRequested value)? restoreFeedRequested,
    TResult Function(NextFeedBatchRequested value)? nextFeedBatchRequested,
    TResult Function(FeedDocumentsClosed value)? feedDocumentsClosed,
    TResult Function(ExcludedSourceAdded value)? excludedSourceAdded,
    TResult Function(ExcludedSourceRemoved value)? excludedSourceRemoved,
    TResult Function(ExcludedSourcesListRequested value)?
        excludedSourcesListRequested,
    TResult Function(DocumentTimeSpent value)? documentTimeSpent,
    TResult Function(UserReactionChanged value)? userReactionChanged,
    TResult Function(SearchRequested value)? searchRequested,
    TResult Function(NextSearchBatchRequested value)? nextSearchBatchRequested,
    TResult Function(RestoreSearchRequested value)? restoreSearchRequested,
    TResult Function(SearchClosed value)? searchClosed,
    TResult Function(SearchTermRequested value)? searchTermRequested,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult maybeWhen<TResult extends Object?>({
    required TResult Function() orElse,
    TResult Function(Configuration configuration, String? aiConfig)? init,
    TResult Function(
      FeedMarkets? feedMarkets,
      int? maxItemsPerFeedBatch,
      int? maxItemsPerSearchBatch,
    )?
        configurationChanged,
    TResult Function()? restoreFeedRequested,
    TResult Function()? nextFeedBatchRequested,
    TResult Function(Set<DocumentId> documentIds)? feedDocumentsClosed,
    TResult Function(String source)? excludedSourceAdded,
    TResult Function(String source)? excludedSourceRemoved,
    TResult Function()? excludedSourcesListRequested,
    TResult Function(DocumentId documentId, DocumentViewMode mode, int seconds)?
        documentTimeSpent,
    TResult Function(DocumentId documentId, UserReaction userReaction)?
        userReactionChanged,
    TResult Function(String queryTerm)? searchRequested,
    TResult Function()? nextSearchBatchRequested,
    TResult Function()? restoreSearchRequested,
    TResult Function()? searchClosed,
    TResult Function()? searchTermRequested,
  }) {
    throw UnimplementedError();
  }

  @override
  Map<String, Object> toJson() {
    throw UnimplementedError();
  }

  @override
  TResult when<TResult extends Object?>({
    required TResult Function(Configuration configuration, String? aiConfig)
        init,
    required TResult Function(
      FeedMarkets? feedMarkets,
      int? maxItemsPerFeedBatch,
      int? maxItemsPerSearchBatch,
    )
        configurationChanged,
    required TResult Function() restoreFeedRequested,
    required TResult Function() nextFeedBatchRequested,
    required TResult Function(Set<DocumentId> documentIds) feedDocumentsClosed,
    required TResult Function(String source) excludedSourceAdded,
    required TResult Function(String source) excludedSourceRemoved,
    required TResult Function() excludedSourcesListRequested,
    required TResult Function(
      DocumentId documentId,
      DocumentViewMode mode,
      int seconds,
    )
        documentTimeSpent,
    required TResult Function(DocumentId documentId, UserReaction reaction)
        userReactionChanged,
    required TResult Function(String queryTerm) searchRequested,
    required TResult Function() nextSearchBatchRequested,
    required TResult Function() restoreSearchRequested,
    required TResult Function() searchClosed,
    required TResult Function() searchTermRequested,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult? whenOrNull<TResult extends Object?>({
    TResult Function(Configuration configuration, String? aiConfig)? init,
    TResult Function(
      FeedMarkets? feedMarkets,
      int? maxItemsPerFeedBatch,
      int? maxItemsPerSearchBatch,
    )?
        configurationChanged,
    TResult Function()? restoreFeedRequested,
    TResult Function()? nextFeedBatchRequested,
    TResult Function(Set<DocumentId> documentIds)? feedDocumentsClosed,
    TResult Function(String source)? excludedSourceAdded,
    TResult Function(String source)? excludedSourceRemoved,
    TResult Function()? excludedSourcesListRequested,
    TResult Function(
      DocumentId documentId,
      DocumentViewMode mode,
      int seconds,
    )?
        documentTimeSpent,
    TResult Function(DocumentId documentId, UserReaction reaction)?
        userReactionChanged,
    TResult Function(String queryTerm)? searchRequested,
    TResult Function()? nextSearchBatchRequested,
    TResult Function()? restoreSearchRequested,
    TResult Function()? searchClosed,
    TResult Function()? searchTermRequested,
  }) {
    throw UnimplementedError();
  }
}

class BadEngineEvent implements EngineEvent {
  const BadEngineEvent();

  @override
  TResult map<TResult extends Object?>({
    required TResult Function(RestoreFeedSucceeded value) restoreFeedSucceeded,
    required TResult Function(RestoreFeedFailed value) restoreFeedFailed,
    required TResult Function(NextFeedBatchRequestSucceeded value)
        nextFeedBatchRequestSucceeded,
    required TResult Function(NextFeedBatchRequestFailed value)
        nextFeedBatchRequestFailed,
    required TResult Function(NextFeedBatchAvailable value)
        nextFeedBatchAvailable,
    required TResult Function(ExcludedSourcesListRequestSucceeded value)
        excludedSourcesListRequestSucceeded,
    required TResult Function(ExcludedSourcesListRequestFailed value)
        excludedSourcesListRequestFailed,
    required TResult Function(FetchingAssetsStarted value)
        fetchingAssetsStarted,
    required TResult Function(FetchingAssetsProgressed value)
        fetchingAssetsProgressed,
    required TResult Function(FetchingAssetsFinished value)
        fetchingAssetsFinished,
    required TResult Function(ClientEventSucceeded value) clientEventSucceeded,
    required TResult Function(EngineExceptionRaised value)
        engineExceptionRaised,
    required TResult Function(DocumentsUpdated value) documentsUpdated,
    required TResult Function(SearchRequestSucceeded value)
        searchRequestSucceeded,
    required TResult Function(SearchRequestFailed value) searchRequestFailed,
    required TResult Function(NextSearchBatchRequestSucceeded value)
        nextSearchBatchRequestSucceeded,
    required TResult Function(NextSearchBatchRequestFailed value)
        nextSearchBatchRequestFailed,
    required TResult Function(RestoreSearchSucceeded value)
        restoreSearchSucceeded,
    required TResult Function(RestoreSearchFailed value) restoreSearchFailed,
    required TResult Function(SearchTermRequestSucceeded value)
        searchTermRequestSucceeded,
    required TResult Function(SearchTermRequestFailed value)
        searchTermRequestFailed,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult? mapOrNull<TResult extends Object?>({
    TResult Function(RestoreFeedSucceeded value)? restoreFeedSucceeded,
    TResult Function(RestoreFeedFailed value)? restoreFeedFailed,
    TResult Function(NextFeedBatchRequestSucceeded value)?
        nextFeedBatchRequestSucceeded,
    TResult Function(NextFeedBatchRequestFailed value)?
        nextFeedBatchRequestFailed,
    TResult Function(NextFeedBatchAvailable value)? nextFeedBatchAvailable,
    TResult Function(ExcludedSourcesListRequestSucceeded value)?
        excludedSourcesListRequestSucceeded,
    TResult Function(ExcludedSourcesListRequestFailed value)?
        excludedSourcesListRequestFailed,
    TResult Function(FetchingAssetsStarted value)? fetchingAssetsStarted,
    TResult Function(FetchingAssetsProgressed value)? fetchingAssetsProgressed,
    TResult Function(FetchingAssetsFinished value)? fetchingAssetsFinished,
    TResult Function(ClientEventSucceeded value)? clientEventSucceeded,
    TResult Function(EngineExceptionRaised value)? engineExceptionRaised,
    TResult Function(DocumentsUpdated value)? documentsUpdated,
    TResult Function(SearchRequestSucceeded value)? searchRequestSucceeded,
    TResult Function(SearchRequestFailed value)? searchRequestFailed,
    TResult Function(NextSearchBatchRequestSucceeded value)?
        nextSearchBatchRequestSucceeded,
    TResult Function(NextSearchBatchRequestFailed value)?
        nextSearchBatchRequestFailed,
    TResult Function(RestoreSearchSucceeded value)? restoreSearchSucceeded,
    TResult Function(RestoreSearchFailed value)? restoreSearchFailed,
    TResult Function(SearchTermRequestSucceeded value)?
        searchTermRequestSucceeded,
    TResult Function(SearchTermRequestFailed value)? searchTermRequestFailed,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult maybeMap<TResult extends Object?>({
    required TResult Function() orElse,
    TResult Function(RestoreFeedSucceeded value)? restoreFeedSucceeded,
    TResult Function(RestoreFeedFailed value)? restoreFeedFailed,
    TResult Function(NextFeedBatchRequestSucceeded value)?
        nextFeedBatchRequestSucceeded,
    TResult Function(NextFeedBatchRequestFailed value)?
        nextFeedBatchRequestFailed,
    TResult Function(NextFeedBatchAvailable value)? nextFeedBatchAvailable,
    TResult Function(ExcludedSourcesListRequestSucceeded value)?
        excludedSourcesListRequestSucceeded,
    TResult Function(ExcludedSourcesListRequestFailed value)?
        excludedSourcesListRequestFailed,
    TResult Function(FetchingAssetsStarted value)? fetchingAssetsStarted,
    TResult Function(FetchingAssetsProgressed value)? fetchingAssetsProgressed,
    TResult Function(FetchingAssetsFinished value)? fetchingAssetsFinished,
    TResult Function(ClientEventSucceeded value)? clientEventSucceeded,
    TResult Function(EngineExceptionRaised value)? engineExceptionRaised,
    TResult Function(DocumentsUpdated value)? documentsUpdated,
    TResult Function(SearchRequestSucceeded value)? searchRequestSucceeded,
    TResult Function(SearchRequestFailed value)? searchRequestFailed,
    TResult Function(NextSearchBatchRequestSucceeded value)?
        nextSearchBatchRequestSucceeded,
    TResult Function(NextSearchBatchRequestFailed value)?
        nextSearchBatchRequestFailed,
    TResult Function(RestoreSearchSucceeded value)? restoreSearchSucceeded,
    TResult Function(RestoreSearchFailed value)? restoreSearchFailed,
    TResult Function(SearchTermRequestSucceeded value)?
        searchTermRequestSucceeded,
    TResult Function(SearchTermRequestFailed value)? searchTermRequestFailed,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult maybeWhen<TResult extends Object?>({
    required TResult Function() orElse,
    TResult Function(List<Document> items)? restoreFeedSucceeded,
    TResult Function(FeedFailureReason reason)? restoreFeedFailed,
    TResult Function(List<Document> items)? nextFeedBatchRequestSucceeded,
    TResult Function(
      FeedFailureReason reason,
      String? message,
    )?
        nextFeedBatchRequestFailed,
    TResult Function()? nextFeedBatchAvailable,
    TResult Function(Set<String> excludedSources)?
        excludedSourcesListRequestSucceeded,
    TResult Function()? excludedSourcesListRequestFailed,
    TResult Function()? fetchingAssetsStarted,
    TResult Function(double percentage)? fetchingAssetsProgressed,
    TResult Function()? fetchingAssetsFinished,
    TResult Function()? clientEventSucceeded,
    TResult Function(
      EngineExceptionReason reason,
      String? message,
      String? stackTrace,
    )?
        engineExceptionRaised,
    TResult Function(List<Document> items)? documentsUpdated,
    TResult Function(ActiveSearch search, List<Document> items)?
        searchRequestSucceeded,
    TResult Function(SearchFailureReason reason)? searchRequestFailed,
    TResult Function(ActiveSearch search, List<Document> items)?
        nextSearchBatchRequestSucceeded,
    TResult Function(SearchFailureReason reason)? nextSearchBatchRequestFailed,
    TResult Function(ActiveSearch search, List<Document> items)?
        restoreSearchSucceeded,
    TResult Function(SearchFailureReason reason)? restoreSearchFailed,
    TResult Function(String searchTerm)? searchTermRequestSucceeded,
    TResult Function(SearchFailureReason reason)? searchTermRequestFailed,
  }) {
    throw UnimplementedError();
  }

  @override
  Map<String, Object> toJson() {
    throw UnimplementedError();
  }

  @override
  TResult when<TResult extends Object?>({
    required TResult Function(List<Document> items) restoreFeedSucceeded,
    required TResult Function(FeedFailureReason reason) restoreFeedFailed,
    required TResult Function(List<Document> items)
        nextFeedBatchRequestSucceeded,
    required TResult Function(
      FeedFailureReason reason,
      String? message,
    )
        nextFeedBatchRequestFailed,
    required TResult Function() nextFeedBatchAvailable,
    required TResult Function(Set<String> excludedSources)
        excludedSourcesListRequestSucceeded,
    required TResult Function() excludedSourcesListRequestFailed,
    required TResult Function() fetchingAssetsStarted,
    required TResult Function(double percentage) fetchingAssetsProgressed,
    required TResult Function() fetchingAssetsFinished,
    required TResult Function() clientEventSucceeded,
    required TResult Function(
      EngineExceptionReason reason,
      String? message,
      String? stackTrace,
    )
        engineExceptionRaised,
    required TResult Function(List<Document> items) documentsUpdated,
    required TResult Function(ActiveSearch search, List<Document> items)
        searchRequestSucceeded,
    required TResult Function(SearchFailureReason reason) searchRequestFailed,
    required TResult Function(ActiveSearch search, List<Document> items)
        nextSearchBatchRequestSucceeded,
    required TResult Function(SearchFailureReason reason)
        nextSearchBatchRequestFailed,
    required TResult Function(ActiveSearch search, List<Document> items)
        restoreSearchSucceeded,
    required TResult Function(SearchFailureReason reason) restoreSearchFailed,
    required TResult Function(String searchTerm) searchTermRequestSucceeded,
    required TResult Function(SearchFailureReason reason)
        searchTermRequestFailed,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult? whenOrNull<TResult extends Object?>({
    TResult Function(List<Document> items)? restoreFeedSucceeded,
    TResult Function(FeedFailureReason reason)? restoreFeedFailed,
    TResult Function(List<Document> items)? nextFeedBatchRequestSucceeded,
    TResult Function(
      FeedFailureReason reason,
      String? message,
    )?
        nextFeedBatchRequestFailed,
    TResult Function()? nextFeedBatchAvailable,
    TResult Function(Set<String> excludedSources)?
        excludedSourcesListRequestSucceeded,
    TResult Function()? excludedSourcesListRequestFailed,
    TResult Function()? fetchingAssetsStarted,
    TResult Function(double percentage)? fetchingAssetsProgressed,
    TResult Function()? fetchingAssetsFinished,
    TResult Function()? clientEventSucceeded,
    TResult Function(
      EngineExceptionReason reason,
      String? message,
      String? stackTrace,
    )?
        engineExceptionRaised,
    TResult Function(List<Document> items)? documentsUpdated,
    TResult Function(ActiveSearch search, List<Document> items)?
        searchRequestSucceeded,
    TResult Function(SearchFailureReason reason)? searchRequestFailed,
    TResult Function(ActiveSearch search, List<Document> items)?
        nextSearchBatchRequestSucceeded,
    TResult Function(SearchFailureReason reason)? nextSearchBatchRequestFailed,
    TResult Function(ActiveSearch search, List<Document> items)?
        restoreSearchSucceeded,
    TResult Function(SearchFailureReason reason)? restoreSearchFailed,
    TResult Function(String searchTerm)? searchTermRequestSucceeded,
    TResult Function(SearchFailureReason reason)? searchTermRequestFailed,
  }) {
    throw UnimplementedError();
  }
}
