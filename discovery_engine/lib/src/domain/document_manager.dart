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

import 'package:xayn_discovery_engine/src/api/events/client_events.dart'
    show DocumentClientEvent;
import 'package:xayn_discovery_engine/src/domain/changed_documents_reporter.dart'
    show ChangedDocumentsReporter;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/domain/models/time_spent.dart'
    show TimeSpent;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/models/user_reacted.dart'
    show UserReacted;
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart'
    show DocumentViewMode;
import 'package:xayn_discovery_engine/src/domain/repository/active_document_repo.dart'
    show ActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/domain/repository/document_repo.dart'
    show DocumentRepository;
import 'package:xayn_discovery_engine/src/domain/repository/engine_state_repo.dart'
    show EngineStateRepository;

/// Business logic concerning the management of documents.
class DocumentManager {
  final Engine _engine;
  final DocumentRepository _documentRepo;
  final ActiveDocumentDataRepository _activeRepo;
  final EngineStateRepository _engineStateRepo;
  final ChangedDocumentsReporter _changedDocsReporter;

  DocumentManager(
    this._engine,
    this._documentRepo,
    this._activeRepo,
    this._engineStateRepo,
    this._changedDocsReporter,
  );

  /// Handle the given document client event.
  ///
  /// Fails if the event [evt] does not have a handler implemented.
  Future<void> handleDocumentClientEvent(DocumentClientEvent evt) =>
      evt.maybeWhen(
        userReactionChanged: (id, reaction) => updateUserReaction(id, reaction),
        documentTimeSpent: (id, mode, sec) =>
            addActiveDocumentTime(id, mode, sec),
        orElse: () =>
            throw UnimplementedError('handler not implemented for $evt'),
      );

  /// Update user reaction for the given document.
  ///
  /// Fails if [id] does not identify an active document.
  Future<void> updateUserReaction(
    DocumentId id,
    UserReaction userReaction,
  ) async {
    final doc = await _documentRepo.fetchById(id);
    if (doc == null || !doc.isActive) {
      throw ArgumentError('id $id does not identify an active document');
    }

    final smbertEmbedding = await _activeRepo.smbertEmbeddingById(id);
    if (smbertEmbedding == null) {
      throw StateError('id $id does not have active data attached');
    }

    await _documentRepo.update(doc..userReaction = userReaction);
    await _engine.userReacted(
      userReaction == UserReaction.positive
          ? await _documentRepo.fetchHistory()
          : null,
      UserReacted(
        id: id,
        stackId: doc.stackId,
        snippet: doc.snippet,
        smbertEmbedding: smbertEmbedding,
        reaction: userReaction,
        market: FeedMarket(
          countryCode: doc.resource.country,
          langCode: doc.resource.language,
        ),
      ),
    );
    await _engineStateRepo.save(await _engine.serialize());
    _changedDocsReporter.notifyChanged([doc]);
  }

  /// Add additional viewing time for the given active document.
  ///
  /// Fails if [sec] is negative or [id] does not identify an active document.
  Future<void> addActiveDocumentTime(
    DocumentId id,
    DocumentViewMode mode,
    int sec,
  ) async {
    if (sec < 0) {
      throw RangeError.range(sec, 0, null);
    }

    final activeData = await _activeRepo.fetchById(id);
    if (activeData == null) {
      throw ArgumentError('id $id does not identify an active document');
    }

    final doc = await _documentRepo.fetchById(id);
    if (doc == null || !doc.isActive) {
      throw ArgumentError('id $id does not identify an active document');
    }

    activeData.addViewTime(mode, Duration(seconds: sec));
    await _activeRepo.update(id, activeData);

    await _engine.timeSpent(
      TimeSpent(
        id: id,
        smbertEmbedding: activeData.smbertEmbedding,
        // As we don't have a `DocumentViewMode` on the Rust side at the moment,
        // we are aggregating Duration from all view modes.
        time: activeData.sumDuration,
        reaction: doc.userReaction,
      ),
    );

    await _engineStateRepo.save(await _engine.serialize());
  }
}
