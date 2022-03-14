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
    show Engine, EngineInitializer;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData, DocumentWithActiveData;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarkets;
import 'package:xayn_discovery_engine/src/domain/models/history.dart'
    show HistoricDocument;
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResource;
import 'package:xayn_discovery_engine/src/domain/models/time_spent.dart'
    show TimeSpent;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/models/user_reacted.dart'
    show UserReacted;

class MockEngine implements Engine {
  late EngineInitializer initializer;
  final Map<String, int> callCounter = {};
  late Document doc0;
  late Document doc1;
  late ActiveDocumentData active0;
  late ActiveDocumentData active1;

  MockEngine([EngineInitializer? initializer]) {
    if (initializer != null) {
      this.initializer = initializer;
    }

    final stackId = StackId();
    doc0 = Document(
      documentId: DocumentId(),
      stackId: stackId,
      batchIndex: 0,
      resource: resource,
    );
    doc1 = Document(
      documentId: DocumentId(),
      stackId: stackId,
      batchIndex: 1,
      resource: resource,
    );
    active1 = ActiveDocumentData(Embedding.fromList([0]));
    active0 = ActiveDocumentData(Embedding.fromList([1, 3]));
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
  Future<Uint8List> serialize() async {
    _incrementCount('serialize');
    return Uint8List(0);
  }

  @override
  Future<void> setMarkets(
    List<HistoricDocument> history,
    FeedMarkets markets,
  ) async {
    _incrementCount('setMarkets');
  }

  @override
  Future<List<DocumentWithActiveData>> getFeedDocuments(
    List<HistoricDocument> history,
    int maxDocuments,
  ) async {
    _incrementCount('getFeedDocuments');

    if (maxDocuments < 1) {
      return [];
    } else if (maxDocuments == 1) {
      return [DocumentWithActiveData(doc0, active0)];
    } else {
      return [
        DocumentWithActiveData(doc0, active0),
        DocumentWithActiveData(doc1, active1),
      ];
    }
  }

  @override
  Future<void> timeSpent(TimeSpent timeSpent) async {
    _incrementCount('timeSpent');
  }

  @override
  Future<void> userReacted(
    List<HistoricDocument>? history,
    UserReacted userReacted,
  ) async {
    _incrementCount('userReacted');
  }

  @override
  Future<List<DocumentWithActiveData>> activeSearch(
    String query,
    int page,
    int pageSize,
  ) async {
    _incrementCount('activeSearch');

    final stackId = StackId.fromBytes(Uint8List.fromList(List.filled(16, 0)));
    final doc0 = Document(
      documentId: DocumentId(),
      stackId: stackId,
      batchIndex: 0,
      resource: resource,
      isSearched: true,
    );
    doc1 = Document(
      documentId: DocumentId(),
      stackId: stackId,
      batchIndex: 1,
      resource: resource,
      isSearched: true,
    );
    active1 = ActiveDocumentData(Embedding.fromList([0]));
    active0 = ActiveDocumentData(Embedding.fromList([1, 3]));

    if (pageSize < 1) {
      return [];
    } else if (pageSize == 1) {
      return [DocumentWithActiveData(doc0, active0)];
    } else {
      return [
        DocumentWithActiveData(doc0, active0),
        DocumentWithActiveData(doc1, active1),
      ];
    }
  }

  @override
  Future<void> dispose() async {
    _incrementCount('dispose');
  }
}

final resource = NewsResource.fromJson(const <String, Object>{
  'title': 'Example',
  'sourceDomain': 'example.com',
  'snippet': 'snippet',
  'url': 'http://exmaple.com/news',
  'datePublished': '1980-01-01T00:00:00.000000',
  'provider': <String, String>{
    'name': 'domain',
    'thumbnail': 'http://thumbnail.example.com',
  },
  'rank': 10,
  'score': 0.1,
  'country': 'EN',
  'language': 'en',
  'topic': 'news',
});
