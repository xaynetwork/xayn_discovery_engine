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

import 'dart:ffi' show nullptr;
import 'dart:typed_data' show Uint8List;

import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine, EngineInitializer;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show DocumentWithActiveData;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket, FeedMarkets;
import 'package:xayn_discovery_engine/src/domain/models/history.dart'
    show HistoricDocument;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show Source, ToStringListExt;
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart';
import 'package:xayn_discovery_engine/src/domain/models/time_spent.dart'
    show TimeSpent;
import 'package:xayn_discovery_engine/src/domain/models/trending_topic.dart'
    show TrendingTopic;
import 'package:xayn_discovery_engine/src/domain/models/user_reacted.dart'
    show UserReacted;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustSharedEngine;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show asyncFfi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/document/document_vec.dart'
    show DocumentSliceFfi;
import 'package:xayn_discovery_engine/src/ffi/types/document/time_spent.dart'
    show TimeSpentFfi;
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reacted.dart'
    show UserReactedFfi;
import 'package:xayn_discovery_engine/src/ffi/types/embedding.dart'
    show EmbeddingFfi;
import 'package:xayn_discovery_engine/src/ffi/types/feed_market.dart'
    show FeedMarketFfi;
import 'package:xayn_discovery_engine/src/ffi/types/feed_market_vec.dart'
    show FeedMarketSliceFfi;
import 'package:xayn_discovery_engine/src/ffi/types/history.dart'
    show HistoricDocumentVecFfi;
import 'package:xayn_discovery_engine/src/ffi/types/init_config.dart'
    show InitConfigFfi;
import 'package:xayn_discovery_engine/src/ffi/types/primitives.dart'
    show Uint8ListFfi;
import 'package:xayn_discovery_engine/src/ffi/types/result.dart'
    show
        resultSharedEngineStringFfiAdapter,
        resultVecDocumentStringFfiAdapter,
        resultVecTrendingTopicStringFfiAdapter,
        resultVecU8StringFfiAdapter,
        resultVoidStringFfiAdapter;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart'
    show StringFfi, StringListFfi;
import 'package:xayn_discovery_engine/src/ffi/types/trending_topic_vec.dart'
    show TrendingTopicSliceFfi;
import 'package:xayn_discovery_engine/src/ffi/types/weighted_source_vec.dart'
    show WeightedSourceListFfi;
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/data_provider.dart'
    show NativeSetupData;

/// A handle to the discovery engine.
class DiscoveryEngineFfi implements Engine {
  final Boxed<RustSharedEngine> _engine;

  const DiscoveryEngineFfi._(final this._engine);

  /// Initializes the engine.
  static Future<DiscoveryEngineFfi> initialize(
    EngineInitializer initializer,
  ) async {
    final setupData = initializer.setupData;
    if (setupData is! NativeSetupData) {
      throw ArgumentError.value(
        setupData,
        'setupData',
        'must be NativeSetupData',
      );
    }
    final result = await asyncFfi.initialize(
      InitConfigFfi(
        initializer.config,
        setupData,
        initializer.trustedSources,
        initializer.excludedSources,
        deConfig: initializer.deConfig,
      ).allocNative().move(),
      initializer.engineState?.allocNative().move() ?? nullptr,
      initializer.history.allocNative().move(),
      initializer.reactedSources.allocVec().move(),
    );
    final boxedEngine = resultSharedEngineStringFfiAdapter.moveNative(result);

    return DiscoveryEngineFfi._(boxedEngine);
  }

  /// Serializes the engine.
  @override
  Future<Uint8List> serialize() async {
    final result = await asyncFfi.serialize(_engine.ref);

    return resultVecU8StringFfiAdapter.consumeNative(result);
  }

  /// Sets the markets.
  @override
  Future<void> setMarkets(
    final List<HistoricDocument> history,
    final List<SourceReacted> sources,
    final FeedMarkets markets,
  ) async {
    final result = await asyncFfi.setMarkets(
      _engine.ref,
      markets.toList().allocVec().move(),
      history.allocNative().move(),
      sources.allocVec().move(),
    );

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  /// Sets the excluded sources.
  @override
  Future<void> setExcludedSources(
    List<HistoricDocument> history,
    List<SourceReacted> sources,
    Set<Source> excluded,
  ) async {
    final result = await asyncFfi.setExcludedSources(
      _engine.ref,
      history.allocNative().move(),
      sources.allocVec().move(),
      excluded.toStringList().allocNative().move(),
    );

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  @override
  Future<void> setTrustedSources(
    List<HistoricDocument> history,
    List<SourceReacted> sources,
    Set<Source> trusted,
  ) async {
    final result = await asyncFfi.setTrustedSources(
      _engine.ref,
      history.allocNative().move(),
      sources.allocVec().move(),
      trusted.toStringList().allocNative().move(),
    );

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  /// Gets feed documents.
  @override
  Future<List<DocumentWithActiveData>> getFeedDocuments(
    final List<HistoricDocument> history,
    final List<SourceReacted> sources,
    final int maxDocuments,
  ) async {
    final result = await asyncFfi.getFeedDocuments(
      _engine.ref,
      history.allocNative().move(),
      sources.allocVec().move(),
      maxDocuments,
    );

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData();
  }

  /// Processes time spent.
  @override
  Future<void> timeSpent(final TimeSpent timeSpent) async {
    final boxedTimeSpent = timeSpent.allocNative();
    await asyncFfi.timeSpent(_engine.ref, boxedTimeSpent.move());
  }

  /// Processes user reaction.
  ///
  /// The history is only required for positive reactions.
  @override
  Future<void> userReacted(
    final List<HistoricDocument>? history,
    final List<SourceReacted> sources,
    final UserReacted userReacted,
  ) async {
    final boxedUserReacted = userReacted.allocNative();
    final result = await asyncFfi.userReacted(
      _engine.ref,
      history?.allocNative().move() ?? nullptr,
      sources.allocVec().move(),
      boxedUserReacted.move(),
    );

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  /// Performs an active search by query.
  @override
  Future<List<DocumentWithActiveData>> searchByQuery(
    String query,
    int page,
    int pageSize,
  ) async {
    final boxedQuery = query.allocNative();
    final result = await asyncFfi.searchByQuery(
      _engine.ref,
      boxedQuery.move(),
      page,
      pageSize,
    );

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData(isSearched: true);
  }

  /// Performs an active search by topic.
  @override
  Future<List<DocumentWithActiveData>> searchByTopic(
    String topic,
    int page,
    int pageSize,
  ) async {
    final boxedTopic = topic.allocNative();
    final result = await asyncFfi.searchByTopic(
      _engine.ref,
      boxedTopic.move(),
      page,
      pageSize,
    );

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData(isSearched: true);
  }

  /// Performs a deep search by term and market.
  ///
  /// The documents are sorted in descending order wrt their cosine similarity towards the
  /// original search term embedding.
  @override
  Future<List<DocumentWithActiveData>> deepSearch(
    String term,
    FeedMarket market,
    Embedding embedding,
  ) async {
    final result = await asyncFfi.deepSearch(
      _engine.ref,
      term.allocNative().move(),
      market.allocNative().move(),
      embedding.allocNative().move(),
    );

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData();
  }

  /// Returns the currently trending topics.
  @override
  Future<List<TrendingTopic>> getTrendingTopics() async {
    final result = await asyncFfi.trendingTopics(_engine.ref);

    return resultVecTrendingTopicStringFfiAdapter
        .consumeNative(result)
        .toTrendingTopicList();
  }

  /// Disposes the engine.
  @override
  Future<void> dispose() async {
    await _engine.free();
  }

  @override
  Future<void> resetAi() async {
    await asyncFfi.resetAi(_engine.ref);
  }
}
