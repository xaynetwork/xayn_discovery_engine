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
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;

/// Interface to Discovery Engine core.
abstract class Engine {
  /// Serializes the state of the [Engine] state.
  Uint8List serialize();

  /// Retrieves at most [maxDocuments] feed documents.
  Map<Document, ActiveDocumentData> getFeedDocuments(int maxDocuments);

  /// Process the feedback about the user spending some time on a document.
  void timeLogged(
    DocumentId docId, {
    required Uint8List smbertEmbedding,
    required Duration seconds,
    required UserReaction reaction,
  });

  /// Process the user's reaction to a document.
  void userReacted(
    DocumentId docId, {
    required StackId stackId,
    required String snippet,
    required Uint8List smbertEmbedding,
    required UserReaction reaction,
  });
}
