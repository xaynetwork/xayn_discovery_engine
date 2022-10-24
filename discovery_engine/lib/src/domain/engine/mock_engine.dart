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
    show DocumentWithActiveData;
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    show ActiveSearch, SearchBy;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarkets;
import 'package:xayn_discovery_engine/src/domain/models/history.dart'
    show HistoricDocument;
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResource;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show Source;
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart';
import 'package:xayn_discovery_engine/src/domain/models/time_spent.dart'
    show TimeSpent;
import 'package:xayn_discovery_engine/src/domain/models/trending_topic.dart'
    show TrendingTopic;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/models/user_reacted.dart'
    show UserReacted;

class MockEngine implements Engine {
  late EngineInitializer initializer;
  final Map<String, int> callCounter = {};
  late List<DocumentWithActiveData> feedDocuments;
  late List<DocumentWithActiveData> activeSearchDocuments;
  late List<DocumentWithActiveData> deepSearchDocuments;
  var trustedSources = <Source>{};
  var excludedSources = <Source>{};

  MockEngine([EngineInitializer? initializer]) {
    if (initializer != null) {
      this.initializer = initializer;
    }
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
  Future<void> configure(String deConfig) async {
    _incrementCount('configure');
  }

  @override
  Future<Uint8List> serialize() async {
    _incrementCount('serialize');
    return Uint8List(0);
  }

  @override
  Future<void> setMarkets(
    List<HistoricDocument> history,
    List<SourceReacted> sources,
    FeedMarkets markets,
  ) async {
    _incrementCount('setMarkets');
  }

  @override
  Future<void> setSources(Set<Source> trusted, Set<Source> excluded) async {
    _incrementCount('setSources');
    trustedSources = trusted;
    excludedSources = excluded;
  }

  @override
  Future<Set<Source>> getExcludedSources() async {
    _incrementCount('getExcludedSources');
    return excludedSources;
  }

  @override
  Future<Set<Source>> getTrustedSources() async {
    _incrementCount('getTrustedSources');
    return trustedSources;
  }

  @override
  Future<void> addExcludedSource(Source excluded) async {
    _incrementCount('addExcludedSource');
  }

  @override
  Future<void> removeExcludedSource(Source excluded) async {
    _incrementCount('removeExcludedSource');
  }

  @override
  Future<void> addTrustedSource(Source trusted) async {
    _incrementCount('addTrustedSource');
  }

  @override
  Future<void> removeTrustedSource(Source trusted) async {
    _incrementCount('removeTrustedSource');
  }

  @override
  Future<List<DocumentWithActiveData>> feedNextBatch() async {
    _incrementCount('feedNextBatch');
    return feedDocuments.take(2).toList(growable: false);
  }

  @override
  Future<List<DocumentWithActiveData>> fed() async {
    _incrementCount('fed');
    return feedDocuments.take(2).toList(growable: false);
  }

  @override
  Future<void> deleteFeedDocuments(Set<DocumentId> ids) async {
    _incrementCount('deleteFeedDocuments');
  }

  @override
  Future<void> timeSpent(TimeSpent timeSpent) async {
    _incrementCount('timeSpent');
  }

  @override
  Future<Document> userReacted(UserReacted userReacted) async {
    _incrementCount('userReacted');
    return Document(
      batchIndex: 0,
      documentId: DocumentId(),
      resource: NewsResource(
        country: 'US',
        datePublished: DateTime.now().toUtc(),
        image: null,
        language: '',
        rank: 0,
        score: null,
        snippet: '',
        sourceDomain: Source('foo.invalid'),
        title: '',
        topic: '',
        url: Uri.parse('https://foo.invalid'),
      ),
      stackId: StackId(),
    );
  }

  @override
  Future<List<DocumentWithActiveData>> searchByQuery(
    String query,
    int page,
  ) async {
    _incrementCount('searchByQuery');
    return activeSearchDocuments.take(20).toList(growable: false);
  }

  @override
  Future<List<DocumentWithActiveData>> searchByTopic(
    String topic,
    int page,
  ) async {
    _incrementCount('searchByTopic');
    return activeSearchDocuments.take(20).toList(growable: false);
  }

  @override
  Future<List<DocumentWithActiveData>> searchById(DocumentId id) async {
    _incrementCount('searchById');
    return deepSearchDocuments;
  }

  @override
  Future<List<DocumentWithActiveData>> searchNextBatch() async {
    _incrementCount('searchNextBatch');
    return activeSearchDocuments.take(20).toList(growable: false);
  }

  @override
  Future<List<DocumentWithActiveData>> searched() async {
    _incrementCount('searched');
    return activeSearchDocuments.take(20).toList(growable: false);
  }

  @override
  Future<ActiveSearch> searchedBy() async {
    _incrementCount('searchedBy');
    return ActiveSearch(
      searchBy: SearchBy.query,
      searchTerm: 'example',
      requestedPageNb: -1,
      pageSize: -1,
    );
  }

  @override
  Future<void> closeSearch() async {
    _incrementCount('closeSearch');
  }

  @override
  Future<List<TrendingTopic>> trendingTopics() async {
    _incrementCount('trendingTopics');
    return [mockTrendingTopic];
  }

  @override
  Future<void> dispose() async {
    _incrementCount('dispose');
  }

  @override
  Future<void> resetAi() async {
    _incrementCount('resetAi');
    return;
  }

  @override
  String? get lastDbOverrideError => null;
}

const mockTrendingTopic = TrendingTopic(
  name: 'Not from Antarctic',
  query: 'Penguins Australia New Zealand',
  image: null,
);
