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

import 'package:hive/hive.dart' show Hive, Box;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/document_repo.dart'
    show DocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show documentBox;

/// Hive repository implementation of [DocumentRepository].
class HiveDocumentRepository implements DocumentRepository {
  Box<Document> get box => Hive.box<Document>(documentBox);

  @override
  Future<Document?> fetchById(DocumentId id) async => box.get(id.toString());

  @override
  Future<List<Document>> fetchByIds(Set<DocumentId> ids) async => <Document>[
        for (final doc in ids.map((id) => box.get(id.toString())))
          if (doc != null) doc
      ];

  @override
  Future<List<Document>> fetchAll() async => box.values.toList();

  @override
  Future<void> update(Document doc) => box.put(doc.documentId.toString(), doc);

  @override
  Future<void> updateMany(Iterable<Document> docs) =>
      box.putAll(<String, Document>{
        for (final doc in docs) doc.documentId.toString(): doc
      });

  @override
  Future<void> removeByIds(Set<DocumentId> ids) async {
    final keys = ids.map((id) => id.toString());
    await box.deleteAll(keys);
  }
}
