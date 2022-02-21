// Copyright 2022 Xayn AG
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

import 'dart:async' show StreamController;
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart'
    show EngineEvent, DocumentsUpdated;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;

/// Class that manages a stream of [DocumentsUpdated] events.
class ChangedDocumentsReporter {
  final _changedDocsCtrl = StreamController<EngineEvent>.broadcast();
  Stream<EngineEvent> get changedDocuments => _changedDocsCtrl.stream;

  void notifyChanged(List<Document> documents) {
    final payload = documents.map((it) => it.toApiDocument()).toList();
    final event = DocumentsUpdated(payload);
    _changedDocsCtrl.add(event);
  }

  Future<void> close() async {
    await _changedDocsCtrl.close();
  }
}
