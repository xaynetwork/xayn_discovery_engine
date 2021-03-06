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
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart';
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
import 'package:xayn_discovery_engine/src/domain/repository/source_reacted_repo.dart';

/// Business logic concerning the management of documents.
class DocumentManager {
  final Engine _engine;
  final DocumentRepository _documentRepo;
  final ActiveDocumentDataRepository _activeRepo;
  final EngineStateRepository _engineStateRepo;
  final ChangedDocumentsReporter _changedDocsReporter;
  final SourceReactedRepository _sourceRepo;

  DocumentManager(
    this._engine,
    this._documentRepo,
    this._activeRepo,
    this._engineStateRepo,
    this._changedDocsReporter,
    this._sourceRepo,
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

    // update reacted sources repo if necessary
    if (userReaction != UserReaction.neutral) {
      final source = doc.resource.sourceDomain;
      final like = userReaction == UserReaction.positive;
      final sourceReacted = await _sourceRepo.fetchBySource(source);

      if (sourceReacted == null) {
        await _sourceRepo.save(SourceReacted(source, like));
      } else if (sourceReacted.liked != like) {
        await _sourceRepo.remove(source);
      } else {
        await _sourceRepo.save(sourceReacted..update());
      }
    }

    await _documentRepo.update(doc..userReaction = userReaction);
    await _engine.userReacted(
      userReaction == UserReaction.positive
          ? await _documentRepo.fetchHistory()
          : null,
      await _sourceRepo.fetchAll(),
      UserReacted(
        id: id,
        stackId: doc.stackId,
        title: doc.resource.title,
        snippet: doc.snippet,
        smbertEmbedding: smbertEmbedding,
        reaction: userReaction,
        market: FeedMarket(
          langCode: doc.resource.language,
          countryCode: doc.resource.country,
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
