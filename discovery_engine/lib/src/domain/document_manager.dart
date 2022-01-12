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
  final DocumentRepository _documentRepo;
  final ActiveDocumentDataRepository _activeRepo;
  final ChangedDocumentRepository _changedRepo;

  DocumentManager(this._documentRepo, this._activeRepo, this._changedRepo);

  /// Handle the given document client event.
  ///
  /// Fails if the event [evt] does not have a handler implemented.
  Future<void> handleDocumentClientEvent(DocumentClientEvent evt) async {
    await evt.maybeWhen(
      documentFeedbackChanged: (id, fdbk) => updateDocumentFeedback(id, fdbk),
      documentTimeLogged: (id, mode, sec) =>
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
    final doc = await _documentRepo.fetchById(id);
    if (doc == null || !doc.isActive) {
      throw ArgumentError('id $id does not identify an active document');
    }
    await _documentRepo.update(doc..feedback = feedback);
  }

  /// Deactivate the given documents.
  Future<void> deactivateDocuments(Set<DocumentId> ids) async {
    await _activeRepo.removeByIds(ids);
    await _changedRepo.removeMany(ids);

    final docs = await _documentRepo.fetchByIds(ids);
    final inactives = docs.map((doc) => doc..isActive = false);
    await _documentRepo.updateMany(inactives);
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
    activeData.addViewTime(mode, Duration(seconds: sec));
    await _activeRepo.update(id, activeData);
  }
}
