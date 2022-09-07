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
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    show ActiveSearch;
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
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
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/models/user_reacted.dart'
    show UserReacted;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustSharedEngine;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show asyncFfi, ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/dart_migration_data.dart';
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
        resultDocumentStringFfiAdapter,
        resultInitializationResultStringFfiAdapter,
        resultSearchStringFfiAdapter,
        resultVecDocumentStringFfiAdapter,
        resultVecStringStringFfiAdapter,
        resultVecTrendingTopicStringFfiAdapter,
        resultVecU8StringFfiAdapter,
        resultVoidStringFfiAdapter;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart'
    show OptionStringFfi, StringFfi, StringListFfi;
import 'package:xayn_discovery_engine/src/ffi/types/trending_topic_vec.dart'
    show TrendingTopicSliceFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart'
    show DocumentIdFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uuid_vec.dart'
    show DocumentIdSetFfi;
import 'package:xayn_discovery_engine/src/ffi/types/weighted_source_vec.dart'
    show WeightedSourceListFfi;
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/data_provider.dart'
    show NativeSetupData;
import 'package:xayn_discovery_engine/src/infrastructure/migration.dart';

/// A handle to the discovery engine.
class DiscoveryEngineFfi implements Engine {
  final Boxed<RustSharedEngine> _engine;
  @override
  final String? lastDbOverrideError;

  const DiscoveryEngineFfi._(this._engine, this.lastDbOverrideError);

  /// Initializes the engine.
  static Future<DiscoveryEngineFfi> initialize(
    EngineInitializer initializer,
    DartMigrationData? dartMigrationData,
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
      dartMigrationData?.allocNative().move() ?? nullptr,
    );
    final boxedInitResult =
        resultInitializationResultStringFfiAdapter.moveNative(result);
    final dbOverrideErr = OptionStringFfi.readNative(
      ffi.initialization_result_place_of_db_override_error(boxedInitResult.ref),
    );
    final boxedEngine = Boxed(
      ffi.destruct_initialization_result_into_shared_engine(
        boxedInitResult.move(),
      ),
      asyncFfi.dispose,
    );
    return DiscoveryEngineFfi._(boxedEngine, dbOverrideErr);
  }

  @override
  Future<void> configure(String deConfig) async {
    await asyncFfi.configure(_engine.ref, deConfig.allocNative().move());
  }

  @override
  Future<Uint8List> serialize() async {
    final result = await asyncFfi.serialize(_engine.ref);

    return resultVecU8StringFfiAdapter.consumeNative(result);
  }

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

  @override
  Future<void> setSources(
    List<SourceReacted> sources, // TODO remove
    Set<Source> excluded,
    Set<Source> trusted,
  ) async {
    final result = await asyncFfi.setSources(
      _engine.ref,
      excluded.toStringList().allocNative().move(),
      trusted.toStringList().allocNative().move(),
    );

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  @override
  Future<Set<Source>> getExcludedSources() async {
    final result = await asyncFfi.getExcludedSources(_engine.ref);

    return resultVecStringStringFfiAdapter
        .consumeNative(result)
        .map((e) => Source(e))
        .toSet();
  }

  @override
  Future<Set<Source>> getTrustedSources() async {
    final result = await asyncFfi.getTrustedSources(_engine.ref);

    return resultVecStringStringFfiAdapter
        .consumeNative(result)
        .map((e) => Source(e))
        .toSet();
  }

  @override
  Future<void> addExcludedSource(
    List<SourceReacted> sources, // TODO remove
    Source excluded,
  ) async {
    final result = await asyncFfi.addExcludedSource(
      _engine.ref,
      excluded.toString().allocNative().move(),
    );

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  @override
  Future<void> removeExcludedSource(
    List<SourceReacted> sources, // TODO remove
    Source excluded,
  ) async {
    final result = await asyncFfi.removeExcludedSource(
      _engine.ref,
      excluded.toString().allocNative().move(),
    );

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  @override
  Future<void> addTrustedSource(
    List<SourceReacted> sources, // TODO remove
    Source trusted,
  ) async {
    final result = await asyncFfi.addTrustedSource(
      _engine.ref,
      trusted.toString().allocNative().move(),
    );

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  @override
  Future<void> removeTrustedSource(
    List<SourceReacted> sources, // TODO remove
    Source trusted,
  ) async {
    final result = await asyncFfi.removeTrustedSource(
      _engine.ref,
      trusted.toString().allocNative().move(),
    );

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  @override
  Future<List<DocumentWithActiveData>> feedNextBatch(
    final List<SourceReacted> sources, // TODO remove
  ) async {
    final result =
        await asyncFfi.feedNextBatch(_engine.ref);

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData();
  }

  @override
  Future<List<DocumentWithActiveData>> getFeedDocuments(
    final List<HistoricDocument> history,
    final List<SourceReacted> sources,
  ) async {
    final result = await asyncFfi.getFeedDocuments(
      _engine.ref,
      history.allocNative().move(),
      sources.allocVec().move(),
    );

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData();
  }

  @override
  Future<List<DocumentWithActiveData>> restoreFeed() async {
    final result = await asyncFfi.restoreFeed(_engine.ref);

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData();
  }

  @override
  Future<void> deleteFeedDocuments(Set<DocumentId> ids) async {
    final result = await asyncFfi.deleteFeedDocuments(
      _engine.ref,
      ids.allocNative().move(),
    );

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

  @override
  Future<void> timeSpent(final TimeSpent timeSpent) async {
    final boxedTimeSpent = timeSpent.allocNative();
    final result = await asyncFfi.timeSpent(_engine.ref, boxedTimeSpent.move());
    resultVoidStringFfiAdapter.consumeNative(result);
  }

  @override
  Future<Document> userReacted(
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

    final doc = resultDocumentStringFfiAdapter.consumeNative(result);

    return doc.toDocument(isSearched: doc.stackId == StackId.nil());
  }

  @override
  Future<List<DocumentWithActiveData>> searchByQuery(
    String query,
    int page,
  ) async {
    final result = await asyncFfi.searchByQuery(
      _engine.ref,
      query.allocNative().move(),
      page,
    );

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData(isSearched: true);
  }

  @override
  Future<List<DocumentWithActiveData>> searchByTopic(
    String topic,
    int page,
  ) async {
    final result = await asyncFfi.searchByTopic(
      _engine.ref,
      topic.allocNative().move(),
      page,
    );

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData(isSearched: true);
  }

  @override
  Future<List<DocumentWithActiveData>> searchById(DocumentId id) async {
    final result =
        await asyncFfi.searchById(_engine.ref, id.allocNative().move());

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData();
  }

  @override
  Future<List<DocumentWithActiveData>> searchNextBatch() async {
    final result = await asyncFfi.searchNextBatch(_engine.ref);

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData(isSearched: true);
  }

  @override
  Future<List<DocumentWithActiveData>> restoreSearch() async {
    final result = await asyncFfi.restoreSearch(_engine.ref);

    return resultVecDocumentStringFfiAdapter
        .consumeNative(result)
        .toDocumentListWithActiveData(isSearched: true);
  }

  @override
  Future<ActiveSearch> searchedBy() async {
    final result = await asyncFfi.searchedBy(_engine.ref);

    return resultSearchStringFfiAdapter.consumeNative(result);
  }

  @override
  Future<void> closeSearch() async {
    final result = await asyncFfi.closeSearch(_engine.ref);

    return resultVoidStringFfiAdapter.consumeNative(result);
  }

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

  @override
  Future<List<TrendingTopic>> trendingTopics() async {
    final result = await asyncFfi.trendingTopics(_engine.ref);

    return resultVecTrendingTopicStringFfiAdapter
        .consumeNative(result)
        .toTrendingTopicList();
  }

  @override
  Future<void> dispose() async {
    await _engine.free();
  }

  @override
  Future<void> resetAi() async {
    await asyncFfi.resetAi(_engine.ref);
  }
}
