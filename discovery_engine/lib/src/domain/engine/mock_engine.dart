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
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket, FeedMarkets;
import 'package:xayn_discovery_engine/src/domain/models/history.dart'
    show HistoricDocument;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show Source;
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart';
import 'package:xayn_discovery_engine/src/domain/models/time_spent.dart'
    show TimeSpent;
import 'package:xayn_discovery_engine/src/domain/models/trending_topic.dart'
    show TrendingTopic;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
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
  Future<void> setExcludedSources(
    List<HistoricDocument> history,
    List<SourceReacted> sources,
    Set<Source> excluded,
  ) async {
    _incrementCount('setExcludedSources');
    excludedSources = excluded;
  }

  @override
  Future<void> setTrustedSources(
    List<HistoricDocument> history,
    List<SourceReacted> sources,
    Set<Source> trusted,
  ) async {
    _incrementCount('setTrustedSources');
    trustedSources = trusted;
  }

  @override
  Future<List<DocumentWithActiveData>> feedNextBatch(
    List<SourceReacted> sources,
    int maxDocuments,
  ) async {
    _incrementCount('feedNextBatch');
    return feedDocuments.take(maxDocuments).toList(growable: false);
  }

  @override
  Future<List<DocumentWithActiveData>> getFeedDocuments(
    List<HistoricDocument> history,
    List<SourceReacted> sources,
    int maxDocuments,
  ) async {
    _incrementCount('getFeedDocuments');
    return feedDocuments.take(maxDocuments).toList(growable: false);
  }

  @override
  Future<List<DocumentWithActiveData>> restoreFeed() async {
    _incrementCount('restoreFeed');
    return feedDocuments.take(10).toList(growable: false);
  }

  @override
  Future<void> timeSpent(TimeSpent timeSpent) async {
    _incrementCount('timeSpent');
  }

  @override
  Future<void> userReacted(
    List<HistoricDocument>? history,
    List<SourceReacted> sources,
    UserReacted userReacted,
  ) async {
    _incrementCount('userReacted');
  }

  @override
  Future<List<DocumentWithActiveData>> searchByQuery(
    String query,
    int page,
    int pageSize,
  ) async {
    _incrementCount('activeSearch');
    return activeSearchDocuments.take(pageSize).toList(growable: false);
  }

  @override
  Future<List<DocumentWithActiveData>> searchByTopic(
    String topic,
    int page,
    int pageSize,
  ) async {
    _incrementCount('searchByTopic');
    return activeSearchDocuments.take(pageSize).toList(growable: false);
  }

  @override
  Future<List<DocumentWithActiveData>> searchById(DocumentId id) async {
    _incrementCount('searchById');
    return deepSearchDocuments;
  }

  @override
  Future<List<DocumentWithActiveData>> searchNextBatch() async {
    _incrementCount('searchNextBatch');
    return activeSearchDocuments.take(10).toList(growable: false);
  }

  @override
  Future<List<DocumentWithActiveData>> restoreSearch() async {
    _incrementCount('searched');
    return activeSearchDocuments.take(10).toList(growable: false);
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
  Future<List<DocumentWithActiveData>> deepSearch(
    String term,
    FeedMarket market,
    Embedding embedding,
  ) async {
    _incrementCount('deepSearch');
    return deepSearchDocuments;
  }

  @override
  Future<List<TrendingTopic>> getTrendingTopics() async {
    _incrementCount('getTrendingTopics');
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
}

const mockTrendingTopic = TrendingTopic(
  name: 'Not from Antarctic',
  query: 'Penguins Australia New Zealand',
  image: null,
);
