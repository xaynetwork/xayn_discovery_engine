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
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/active_document_repo.dart'
    show ActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show activeDocumentDataBox;

/// Hive repository implementation of [ActiveDocumentDataRepository].
class HiveActiveDocumentDataRepository implements ActiveDocumentDataRepository {
  Box<ActiveDocumentData> get box =>
      Hive.box<ActiveDocumentData>(activeDocumentDataBox);

  @override
  Future<ActiveDocumentData?> fetchById(DocumentId id) async =>
      box.get(id.toString());

  @override
  Future<Embedding?> smbertEmbeddingById(DocumentId id) async =>
      box.get(id.toString())?.smbertEmbedding;

  @override
  Future<void> update(DocumentId id, ActiveDocumentData data) =>
      box.put(id.toString(), data);

  @override
  Future<void> removeByIds(Iterable<DocumentId> ids) =>
      box.deleteAll(ids.map<String>((id) => id.toString()));

  @override
  Future<void> clearAIState() => box.clear();
}
