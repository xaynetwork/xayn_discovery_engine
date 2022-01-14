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
        ClientEvent,
        ClientEventSucceeded,
        Configuration,
        ConfigurationChanged,
        Document,
        DocumentFeedback,
        DocumentViewMode,
        DocumentFeedbackChanged,
        DocumentId,
        DocumentTimeLogged,
        EngineEvent,
        EngineExceptionRaised,
        EngineExceptionReason,
        FeedDocumentsClosed,
        FeedFailureReason,
        FeedRequestFailed,
        FeedRequestSucceeded,
        FeedRequested,
        Init,
        NextFeedBatchAvailable,
        NextFeedBatchRequestFailed,
        NextFeedBatchRequestSucceeded,
        NextFeedBatchRequested,
        ResetEngine,
        FeedMarkets;

class BadClientEvent implements ClientEvent {
  const BadClientEvent();

  @override
  TResult map<TResult extends Object?>({
    required TResult Function(Init value) init,
    required TResult Function(ResetEngine value) resetEngine,
    required TResult Function(ConfigurationChanged value) configurationChanged,
    required TResult Function(FeedRequested value) feedRequested,
    required TResult Function(NextFeedBatchRequested value)
        nextFeedBatchRequested,
    required TResult Function(FeedDocumentsClosed value) feedDocumentsClosed,
    required TResult Function(DocumentTimeLogged value) documentTimeLogged,
    required TResult Function(DocumentFeedbackChanged value)
        documentFeedbackChanged,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult? mapOrNull<TResult extends Object?>({
    TResult Function(Init value)? init,
    TResult Function(ResetEngine value)? resetEngine,
    TResult Function(ConfigurationChanged value)? configurationChanged,
    TResult Function(FeedRequested value)? feedRequested,
    TResult Function(NextFeedBatchRequested value)? nextFeedBatchRequested,
    TResult Function(FeedDocumentsClosed value)? feedDocumentsClosed,
    TResult Function(DocumentTimeLogged value)? documentTimeLogged,
    TResult Function(DocumentFeedbackChanged value)? documentFeedbackChanged,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult maybeMap<TResult extends Object?>({
    required TResult Function() orElse,
    TResult Function(Init value)? init,
    TResult Function(ResetEngine value)? resetEngine,
    TResult Function(ConfigurationChanged value)? configurationChanged,
    TResult Function(FeedRequested value)? feedRequested,
    TResult Function(NextFeedBatchRequested value)? nextFeedBatchRequested,
    TResult Function(FeedDocumentsClosed value)? feedDocumentsClosed,
    TResult Function(DocumentTimeLogged value)? documentTimeLogged,
    TResult Function(DocumentFeedbackChanged value)? documentFeedbackChanged,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult maybeWhen<TResult extends Object?>({
    required TResult Function() orElse,
    TResult Function(Configuration configuration)? init,
    TResult Function()? resetEngine,
    TResult Function(
      FeedMarkets? feedMarkets,
      int? maxItemsPerFeedBatch,
    )?
        configurationChanged,
    TResult Function()? feedRequested,
    TResult Function()? nextFeedBatchRequested,
    TResult Function(Set<DocumentId> documentIds)? feedDocumentsClosed,
    TResult Function(DocumentId documentId, DocumentViewMode mode, int seconds)?
        documentTimeLogged,
    TResult Function(DocumentId documentId, DocumentFeedback feedback)?
        documentFeedbackChanged,
  }) {
    throw UnimplementedError();
  }

  @override
  Map<String, Object> toJson() {
    throw UnimplementedError();
  }

  @override
  TResult when<TResult extends Object?>({
    required TResult Function(Configuration configuration) init,
    required TResult Function() resetEngine,
    required TResult Function(
      FeedMarkets? feedMarkets,
      int? maxItemsPerFeedBatch,
    )
        configurationChanged,
    required TResult Function() feedRequested,
    required TResult Function() nextFeedBatchRequested,
    required TResult Function(Set<DocumentId> documentIds) feedDocumentsClosed,
    required TResult Function(
      DocumentId documentId,
      DocumentViewMode mode,
      int seconds,
    )
        documentTimeLogged,
    required TResult Function(DocumentId documentId, DocumentFeedback feedback)
        documentFeedbackChanged,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult? whenOrNull<TResult extends Object?>({
    TResult Function(Configuration configuration)? init,
    TResult Function()? resetEngine,
    TResult Function(
      FeedMarkets? feedMarkets,
      int? maxItemsPerFeedBatch,
    )?
        configurationChanged,
    TResult Function()? feedRequested,
    TResult Function()? nextFeedBatchRequested,
    TResult Function(Set<DocumentId> documentIds)? feedDocumentsClosed,
    TResult Function(
      DocumentId documentId,
      DocumentViewMode mode,
      int seconds,
    )?
        documentTimeLogged,
    TResult Function(DocumentId documentId, DocumentFeedback feedback)?
        documentFeedbackChanged,
  }) {
    throw UnimplementedError();
  }
}

class BadEngineEvent implements EngineEvent {
  const BadEngineEvent();

  @override
  TResult map<TResult extends Object?>({
    required TResult Function(FeedRequestSucceeded value) feedRequestSucceeded,
    required TResult Function(FeedRequestFailed value) feedRequestFailed,
    required TResult Function(NextFeedBatchRequestSucceeded value)
        nextFeedBatchRequestSucceeded,
    required TResult Function(NextFeedBatchRequestFailed value)
        nextFeedBatchRequestFailed,
    required TResult Function(NextFeedBatchAvailable value)
        nextFeedBatchAvailable,
    required TResult Function(ClientEventSucceeded value) clientEventSucceeded,
    required TResult Function(EngineExceptionRaised value)
        engineExceptionRaised,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult? mapOrNull<TResult extends Object?>({
    TResult Function(FeedRequestSucceeded value)? feedRequestSucceeded,
    TResult Function(FeedRequestFailed value)? feedRequestFailed,
    TResult Function(NextFeedBatchRequestSucceeded value)?
        nextFeedBatchRequestSucceeded,
    TResult Function(NextFeedBatchRequestFailed value)?
        nextFeedBatchRequestFailed,
    TResult Function(NextFeedBatchAvailable value)? nextFeedBatchAvailable,
    TResult Function(ClientEventSucceeded value)? clientEventSucceeded,
    TResult Function(EngineExceptionRaised value)? engineExceptionRaised,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult maybeMap<TResult extends Object?>({
    required TResult Function() orElse,
    TResult Function(FeedRequestSucceeded value)? feedRequestSucceeded,
    TResult Function(FeedRequestFailed value)? feedRequestFailed,
    TResult Function(NextFeedBatchRequestSucceeded value)?
        nextFeedBatchRequestSucceeded,
    TResult Function(NextFeedBatchRequestFailed value)?
        nextFeedBatchRequestFailed,
    TResult Function(NextFeedBatchAvailable value)? nextFeedBatchAvailable,
    TResult Function(ClientEventSucceeded value)? clientEventSucceeded,
    TResult Function(EngineExceptionRaised value)? engineExceptionRaised,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult maybeWhen<TResult extends Object?>({
    required TResult Function() orElse,
    TResult Function(List<Document> items)? feedRequestSucceeded,
    TResult Function(FeedFailureReason reason)? feedRequestFailed,
    TResult Function(List<Document> items)? nextFeedBatchRequestSucceeded,
    TResult Function(FeedFailureReason reason)? nextFeedBatchRequestFailed,
    TResult Function()? nextFeedBatchAvailable,
    TResult Function()? clientEventSucceeded,
    TResult Function(EngineExceptionReason reason)? engineExceptionRaised,
  }) {
    throw UnimplementedError();
  }

  @override
  Map<String, Object> toJson() {
    throw UnimplementedError();
  }

  @override
  TResult when<TResult extends Object?>({
    required TResult Function(List<Document> items) feedRequestSucceeded,
    required TResult Function(FeedFailureReason reason) feedRequestFailed,
    required TResult Function(List<Document> items)
        nextFeedBatchRequestSucceeded,
    required TResult Function(FeedFailureReason reason)
        nextFeedBatchRequestFailed,
    required TResult Function() nextFeedBatchAvailable,
    required TResult Function() clientEventSucceeded,
    required TResult Function(EngineExceptionReason reason)
        engineExceptionRaised,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult? whenOrNull<TResult extends Object?>({
    TResult Function(List<Document> items)? feedRequestSucceeded,
    TResult Function(FeedFailureReason reason)? feedRequestFailed,
    TResult Function(List<Document> items)? nextFeedBatchRequestSucceeded,
    TResult Function(FeedFailureReason reason)? nextFeedBatchRequestFailed,
    TResult Function()? nextFeedBatchAvailable,
    TResult Function()? clientEventSucceeded,
    TResult Function(EngineExceptionReason reason)? engineExceptionRaised,
  }) {
    throw UnimplementedError();
  }
}
