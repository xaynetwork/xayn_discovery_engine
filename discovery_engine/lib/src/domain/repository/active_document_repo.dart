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

import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Repository interface for additional data relating to active documents.
abstract class ActiveDocumentDataRepository {
  /// Fetch active document data by id.
  Future<ActiveDocumentData?> fetchById(DocumentId id);

  /// Fetch the SMBert embedding associated with the given document.
  Future<Uint8List?> smbertEmbeddingById(DocumentId id);

  /// Update data associated with the given document.
  ///
  /// [id] is assumed to identify an active document.
  Future<void> update(DocumentId id, ActiveDocumentData data);

  /// Remove data associated with the given active documents.
  Future<void> removeByIds(Iterable<DocumentId> ids);
}
