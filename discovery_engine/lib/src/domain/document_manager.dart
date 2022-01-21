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
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show DocumentFeedback;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart'
    show DocumentViewMode;
import 'package:xayn_discovery_engine/src/domain/repository/active_document_repo.dart'
    show ActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/domain/repository/changed_document_repo.dart'
    show ChangedDocumentRepository;
import 'package:xayn_discovery_engine/src/domain/repository/document_repo.dart'
    show DocumentRepository;

/// Business logic concerning the management of documents.
class DocumentManager {
  final Engine _engine;
  final DocumentRepository documentRepo;
  final ActiveDocumentDataRepository activeRepo;
  final ChangedDocumentRepository _changedRepo;

  DocumentManager(
    this._engine,
    this.documentRepo,
    this.activeRepo,
    this._changedRepo,
  );

  /// Handle the given document client event.
  ///
  /// Fails if the event [evt] does not have a handler implemented.
  Future<void> handleDocumentClientEvent(DocumentClientEvent evt) async {
    await evt.maybeWhen(
      documentFeedbackChanged: (id, fdbk) => updateDocumentFeedback(id, fdbk),
      documentTimeSpent: (id, mode, sec) =>
          addActiveDocumentTime(id, mode, sec),
      orElse: throw UnimplementedError('handler not implemented for $evt'),
    );
  }

  /// Update feedback for the given document.
  ///
  /// Fails if [id] does not identify an active document.
  Future<void> updateDocumentFeedback(
    DocumentId id,
    DocumentFeedback feedback,
  ) async {
    final doc = await documentRepo.fetchById(id);
    if (doc == null || !doc.isActive) {
      throw ArgumentError('id $id does not identify an active document');
    }
    await documentRepo.update(doc..feedback = feedback);
    final smbertEmbedding = await activeRepo.smbertEmbeddingById(id);
    if (smbertEmbedding == null) {
      throw ArgumentError('id $id does not have active data attached');
    }
    _engine.userReacted(
      id,
      stackId: '',
      smbertEmbedding: smbertEmbedding,
      reaction: feedback,
    );
  }

  /// Deactivate the given documents.
  Future<void> deactivateDocuments(Set<DocumentId> ids) async {
    await activeRepo.removeByIds(ids);
    await _changedRepo.removeMany(ids);

    final docs = await documentRepo.fetchByIds(ids);
    final inactives = docs.map((doc) => doc..isActive = false);
    await documentRepo.updateMany(inactives);
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
    final activeData = await activeRepo.fetchById(id);
    if (activeData == null) {
      throw ArgumentError('id $id does not identify an active document');
    }
    activeData.addViewTime(mode, Duration(seconds: sec));
    await activeRepo.update(id, activeData);

    // As we don't have a `DocumentViewMode` on the Rust side at the moment,
    // we need to decide which value to use or to aggregate all view modes.
    final sumDuration = activeData.viewTime.values
        .reduce((aggregate, duration) => aggregate + duration);

    _engine.timeLogged(
      id,
      smbertEmbedding: activeData.smbertEmbedding,
      seconds: sumDuration,
    );
  }
}
