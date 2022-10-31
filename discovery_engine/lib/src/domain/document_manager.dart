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

import 'package:xayn_discovery_engine/discovery_engine.dart'
    show DocumentClientEvent, DocumentId, DocumentViewMode, UserReaction;
import 'package:xayn_discovery_engine/src/domain/changed_documents_reporter.dart'
    show ChangedDocumentsReporter;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/models/time_spent.dart'
    show TimeSpent;
import 'package:xayn_discovery_engine/src/domain/models/user_reacted.dart'
    show UserReacted;

/// Business logic concerning the management of documents.
class DocumentManager {
  final Engine _engine;
  final ChangedDocumentsReporter _changedDocsReporter;

  DocumentManager(this._engine, this._changedDocsReporter);

  /// Handle the given document client event.
  ///
  /// Fails if the event [evt] does not have a handler implemented.
  Future<void> handleDocumentClientEvent(DocumentClientEvent evt) =>
      evt.maybeWhen(
        userReactionChanged: updateUserReaction,
        documentTimeSpent: addActiveDocumentTime,
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
    final document =
        await _engine.userReacted(UserReacted(id: id, reaction: userReaction));
    _changedDocsReporter.notifyChanged([document]);
  }

  /// Add additional viewing time for the given active document.
  ///
  /// Fails if [viewTimeSecs] is negative or [id] does not identify an active document.
  Future<void> addActiveDocumentTime(
    DocumentId id,
    DocumentViewMode viewMode,
    int viewTimeSecs,
  ) async {
    if (viewTimeSecs < 0) {
      throw RangeError.range(viewTimeSecs, 0, null);
    }

    await _engine.timeSpent(
      TimeSpent(
        id: id,
        viewTime: Duration(seconds: viewTimeSecs),
        viewMode: viewMode,
      ),
    );
  }
}
