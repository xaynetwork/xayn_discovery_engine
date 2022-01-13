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

import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Repository interface for ids of documents whose status has changed since the
/// previous call of the feedback loop.
abstract class ChangedDocumentRepository {
  /// Fetch all the document ids.
  Future<List<DocumentId>> fetchAll();

  /// Add the id of a changed document.
  ///
  /// This has no effect if [id] has already been added.
  Future<void> add(DocumentId id);

  /// Clear the repository.
  Future<void> removeAll();

  /// Remove the given document ids.
  Future<void> removeMany(Iterable<DocumentId> ids);
}
