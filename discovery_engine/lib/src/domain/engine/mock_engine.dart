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

import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, DocumentFeedback;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/models/web_resource.dart'
    show WebResource;

class MockEngine implements Engine {
  final Map<String, int> callCounter = {};
  late Document doc0;
  late Document doc1;
  late ActiveDocumentData active0;
  late ActiveDocumentData active1;

  MockEngine() {
    final resource = WebResource.fromJson(const <String, Object>{
      'title': 'Example',
      'displayUrl': 'domain.com',
      'snippet': 'snippet',
      'url': 'http://domain.com',
      'datePublished': '1980-01-01T00:00:00.000000',
      'provider': <String, String>{
        'name': 'domain',
        'thumbnail': 'http://thumbnail.domain.com',
      },
    });
    final stackId = StackId();

    doc0 = Document(
      stackId: stackId,
      personalizedRank: 0,
      webResource: resource,
    );
    doc1 = Document(
      stackId: stackId,
      personalizedRank: 1,
      webResource: resource,
    );
    active0 = ActiveDocumentData(Uint8List(0));
    active1 = ActiveDocumentData(Uint8List(1));
  }

  void _incrementCount(String key) {
    final count = getCallCount(key);
    callCounter[key] = count + 1;
  }

  int getCallCount(String key) {
    return callCounter[key] ?? 0;
  }

  void resetCallCounter() {
    callCounter.clear();
  }

  @override
  Map<Document, ActiveDocumentData> getFeedDocuments(int maxDocuments) {
    _incrementCount('getFeedDocuments');

    if (maxDocuments < 1) {
      return {};
    } else if (maxDocuments == 1) {
      return {doc0: active0};
    } else {
      return {doc0: active0, doc1: active1};
    }
  }

  @override
  void timeLogged(
    DocumentId docId, {
    required Uint8List smbertEmbedding,
    required Duration seconds,
  }) {
    _incrementCount('timeLogged');
  }

  @override
  void userReacted(
    DocumentId docId, {
    required StackId stackId,
    required String snippet,
    required Uint8List smbertEmbedding,
    required DocumentFeedback reaction,
  }) {
    _incrementCount('userReacted');
  }
}
