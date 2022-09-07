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

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:xayn_discovery_engine/src/domain/assets/data_provider.dart'
    show SetupData;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show DocumentWithActiveData;
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    show ActiveSearch;
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
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

/// Interface to Discovery Engine core.
abstract class Engine {
  /// Returns the intermediate error which caused a db reset when initializing the engine.
  String? get lastDbOverrideError;

  /// Serializes the state of the [Engine].
  Future<Uint8List> serialize();

  /// Configures the running engine.
  Future<void> configure(String deConfig);

  /// Changes the currently supported markets.
  Future<void> setMarkets(
    List<HistoricDocument> history,
    List<SourceReacted> sources,
    FeedMarkets markets,
  );

  /// Changes the currently excluded sources.
  Future<void> setExcludedSources(
    List<HistoricDocument> history,
    List<SourceReacted> sources,
    Set<Source> excluded,
  );

  /// Changes the trusted sources.
  Future<void> setTrustedSources(
    List<HistoricDocument> history,
    List<SourceReacted> sources,
    Set<Source> trusted,
  );

  /// Changes the excluded and trusted sources.
  Future<void> setSources(
    Set<Source> excluded,
    Set<Source> trusted,
  );

  /// Returns the excluded sources.
  Future<Set<Source>> getExcludedSources();

  /// Returns the trusted sources.
  Future<Set<Source>> getTrustedSources();

  /// Adds an excluded source.
  Future<void> addExcludedSource(
    Source excluded,
  );

  /// Removes an excluded source.
  Future<void> removeExcludedSource(
    Source excluded,
  );

  /// Adds a trusted source.
  Future<void> addTrustedSource(
    Source trusted,
  );

  /// Removes a trusted source.
  Future<void> removeTrustedSource(
    Source trusted,
  );

  /// Gets the next batch of feed documents.
  Future<List<DocumentWithActiveData>> feedNextBatch();

  /// Gets the next batch of feed documents.
  Future<List<DocumentWithActiveData>> getFeedDocuments(
    List<HistoricDocument> history,
    List<SourceReacted> sources,
  );

  /// Restores the feed documents, ordered by their global rank (timestamp & local rank).
  Future<List<DocumentWithActiveData>> restoreFeed();

  /// Deletes the feed documents.
  Future<void> deleteFeedDocuments(Set<DocumentId> ids);

  /// Process the feedback about the user spending some time on a document.
  Future<void> timeSpent(TimeSpent timeSpent);

  /// Process the user's reaction to a document.
  ///
  /// The history is only required if the reaction is positive and if
  /// `cfgFeatureStorage` is disabled.
  ///
  /// The returned `Document` will only be consistent if `cfgFeatureStorage`
  /// is enabled. The history and most fields of `UserReacted` can
  /// be empty/dummy data if `cfgFeatureStorage` feature is enabled.
  Future<Document> userReacted(
    List<HistoricDocument>? history,
    List<SourceReacted> sources,
    UserReacted userReacted,
  );

  /// Perform an active search by query.
  Future<List<DocumentWithActiveData>> searchByQuery(String query, int page);

  /// Perform an active search by topic.
  Future<List<DocumentWithActiveData>> searchByTopic(String topic, int page);

  /// Performs an active search by document id (aka deep search).
  ///
  /// The documents are sorted in descending order wrt their cosine similarity towards the
  /// original search term embedding.
  Future<List<DocumentWithActiveData>> searchById(DocumentId id);

  /// Gets the next batch of the current active search.
  Future<List<DocumentWithActiveData>> searchNextBatch();

  /// Restores the current active search, ordered by their global rank (timestamp & local rank).
  Future<List<DocumentWithActiveData>> restoreSearch();

  /// Gets the current active search mode and term.
  Future<ActiveSearch> searchedBy();

  /// Closes the current active search.
  Future<void> closeSearch();

  /// Performs a deep search by term and market.
  ///
  /// The documents are sorted in descending order wrt their cosine similarity towards the
  /// original search term embedding.
  Future<List<DocumentWithActiveData>> deepSearch(
    String term,
    FeedMarket market,
    Embedding embedding,
  );

  /// Returns the currently trending topics.
  Future<List<TrendingTopic>> trendingTopics();

  /// Disposes the engine.
  Future<void> dispose();

  /// Resets the AI state of the engine.
  Future<void> resetAi();
}

/// Passed to constructors/initializers of `Engine` implementing classes.
class EngineInitializer with EquatableMixin {
  /// The general configuration of the discovery engine.
  final Configuration config;

  /// The data used to bootstrap it.
  final SetupData setupData;

  /// The state to restore.
  final Uint8List? engineState;

  /// The history to use for filtering initial results.
  final List<HistoricDocument> history;

  // Information about previously reacted sources.
  final List<SourceReacted> reactedSources; // TODO maybe Set<>

  /// An opaque encoded configuration for the DE.
  final String? deConfig;

  /// A set of favourite sources.
  final Set<Source> trustedSources;

  /// A set of excluded sources.
  final Set<Source> excludedSources;

  EngineInitializer({
    required this.config,
    required this.setupData,
    required this.engineState,
    required this.history,
    required this.reactedSources,
    required this.deConfig,
    required this.trustedSources,
    required this.excludedSources,
  });

  @override
  List<Object?> get props => [
        config,
        setupData,
        engineState,
        history,
        reactedSources,
        deConfig,
        trustedSources,
        excludedSources,
      ];
}
