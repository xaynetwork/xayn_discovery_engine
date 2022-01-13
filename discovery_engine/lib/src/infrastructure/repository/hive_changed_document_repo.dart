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

import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart' show Hive, Box;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/changed_document_repo.dart'
    show ChangedDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show changedDocumentIdBox;

/// Hive repository implementation of [ChangedDocumentRepository].
class HiveChangedDocumentRepository implements ChangedDocumentRepository {
  Box<Uint8List> get box => Hive.box<Uint8List>(changedDocumentIdBox);

  @override
  Future<List<DocumentId>> fetchAll() async =>
      box.values.map((bytes) => DocumentId.fromBytes(bytes)).toList();

  @override
  Future<void> add(DocumentId id) => box.put(id.toString(), id.value);

  @override
  Future<void> removeAll() => box.clear();

  @override
  Future<void> removeMany(Iterable<DocumentId> ids) =>
      box.deleteAll(ids.map<String>((id) => id.toString()));
}
