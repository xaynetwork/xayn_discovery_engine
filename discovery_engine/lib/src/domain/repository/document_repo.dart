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

import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Document repository interface.
abstract class DocumentRepository {
  /// Fetch document by id.
  Future<Document?> fetchById(DocumentId id);

  /// Fetch documents by ids.
  ///
  /// Any id that does not identify a document is ignored.
  Future<List<Document>> fetchByIds(Set<DocumentId> ids);

  /// Fetch all documents.
  Future<List<Document>> fetchAll();

  /// Update with the given document.
  Future<void> update(Document doc);

  /// Update with the given documents.
  ///
  /// If [docs] contains multiple documents with the same id, the last
  /// occurrence with that id will overwrite previous occurrences.
  Future<void> updateMany(Iterable<Document> docs);
}
