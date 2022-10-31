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

import 'package:xayn_discovery_engine/src/api/events/client_events.dart'
    show FeedClientEvent;
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart'
    show EngineEvent, EngineExceptionReason, FeedFailureReason;
import 'package:xayn_discovery_engine/src/api/models/document.dart'
    show DocumentApiConversion;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show DocumentWithActiveData;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show AvailableSources, Source;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Business logic concerning the management of the feed.
class FeedManager {
  final Engine _engine;
  final AvailableSources _availableSources;

  FeedManager(this._engine, this._availableSources);

  /// Handle the given feed client event.
  ///
  /// Fails if [event] does not have a handler implemented.
  Future<EngineEvent> handleFeedClientEvent(FeedClientEvent event) =>
      event.maybeWhen(
        restoreFeedRequested: restoreFeed,
        nextFeedBatchRequested: nextFeedBatch,
        feedDocumentsClosed: deactivateDocuments,
        excludedSourceAdded: addExcludedSource,
        excludedSourceRemoved: removeExcludedSource,
        excludedSourcesListRequested: getExcludedSourcesList,
        trustedSourceAdded: addTrustedSource,
        trustedSourceRemoved: removeTrustedSource,
        trustedSourcesListRequested: getTrustedSourcesList,
        availableSourcesListRequested: getAvailableSourcesList,
        setSourcesRequested: setSources,
        orElse: () =>
            throw UnimplementedError('handler not implemented for $event'),
      );

  /// Generates the feed of active documents, ordered by their global rank.
  ///
  /// That is, documents are ordered by their timestamp, then local rank.
  Future<EngineEvent> restoreFeed() async {
    final List<DocumentWithActiveData> docs;
    try {
      docs = await _engine.restoreFeed();
    } on Exception {
      return const EngineEvent.restoreFeedFailed(FeedFailureReason.dbError);
    }

    return EngineEvent.restoreFeedSucceeded(
      docs.map((doc) => doc.document.toApiRepr()).toList(),
    );
  }

  /// Obtain the next batch of feed documents and persist to repositories.
  Future<EngineEvent> nextFeedBatch() async {
    final List<DocumentWithActiveData> docs;
    try {
      docs = await _engine.feedNextBatch();
    } on Exception catch (e) {
      return EngineEvent.nextFeedBatchRequestFailed(
        FeedFailureReason.stacksOpsError,
        errors: '$e',
      );
    }

    if (docs.isEmpty) {
      return const EngineEvent.nextFeedBatchRequestFailed(
        FeedFailureReason.noNewsForMarket,
      );
    }

    return EngineEvent.nextFeedBatchRequestSucceeded(
      docs.map((doc) => doc.document.toApiRepr()).toList(),
    );
  }

  /// Deactivate the given documents.
  Future<EngineEvent> deactivateDocuments(Set<DocumentId> ids) async {
    try {
      await _engine.deleteFeedDocuments(ids);
    } on Exception catch (e) {
      return EngineEvent.engineExceptionRaised(
        EngineExceptionReason.genericError,
        message: '$e',
      );
    }

    return const EngineEvent.clientEventSucceeded();
  }

  /// Override current trusted and excluded sources with the new ones provided
  /// by the client.
  Future<EngineEvent> setSources(
    Set<Source> trustedSources,
    Set<Source> excludedSources,
  ) async {
    final duplicates = trustedSources.intersection(excludedSources);
    if (duplicates.isNotEmpty) {
      return EngineEvent.setSourcesRequestFailed(duplicates);
    }

    await _engine.setSources(trustedSources, excludedSources);

    return EngineEvent.setSourcesRequestSucceeded(
      trustedSources: trustedSources,
      excludedSources: excludedSources,
    );
  }

  /// Adds an excluded source.
  Future<EngineEvent> addExcludedSource(Source source) async {
    await _engine.addExcludedSource(source);

    return EngineEvent.addExcludedSourceRequestSucceeded(source);
  }

  /// Removes an excluded source.
  Future<EngineEvent> removeExcludedSource(Source source) async {
    await _engine.removeExcludedSource(source);

    return EngineEvent.removeExcludedSourceRequestSucceeded(source);
  }

  /// Returns the excluded sources.
  Future<EngineEvent> getExcludedSourcesList() async {
    final sources = await _engine.getExcludedSources();

    return EngineEvent.excludedSourcesListRequestSucceeded(sources);
  }

  /// Adds a trusted source.
  Future<EngineEvent> addTrustedSource(Source source) async {
    await _engine.addTrustedSource(source);

    return EngineEvent.addTrustedSourceRequestSucceeded(source);
  }

  /// Removes a trusted source.
  Future<EngineEvent> removeTrustedSource(Source source) async {
    await _engine.removeTrustedSource(source);

    return EngineEvent.removeTrustedSourceRequestSucceeded(source);
  }

  /// Returns the trusted sources.
  Future<EngineEvent> getTrustedSourcesList() async {
    final sources = await _engine.getTrustedSources();

    return EngineEvent.trustedSourcesListRequestSucceeded(sources);
  }

  Future<EngineEvent> getAvailableSourcesList(String fuzzySearchTerm) async {
    final sources = _availableSources
        .search(fuzzySearchTerm)
        .map((source) => source.item)
        .toList(growable: false);

    if (sources.isEmpty) {
      return const EngineEvent.availableSourcesListRequestFailed();
    }

    return EngineEvent.availableSourcesListRequestSucceeded(sources);
  }
}
