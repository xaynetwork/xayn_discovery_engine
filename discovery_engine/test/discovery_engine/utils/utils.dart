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

import 'dart:isolate' show SendPort;

import 'package:xayn_discovery_engine/src/api/api.dart'
    show Configuration, EngineEvent, FeedMarket;
import 'package:xayn_discovery_engine/src/api/models/active_search.dart'
    show ActiveSearchApiConversion;
import 'package:xayn_discovery_engine/src/discovery_engine_base.dart'
    show DiscoveryEngine;
import 'package:xayn_discovery_engine/src/discovery_engine_worker.dart'
    show DiscoveryEngineWorker;
import 'package:xayn_discovery_engine/src/domain/assets/assets.dart'
    show Manifest;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData, DocumentWithActiveData;
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    show ActiveSearch, SearchBy;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResource;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show mockedAvailableSources, Source;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;

import '../../assets/utils/mock_manifest_reader.dart';

typedef EntryPoint = void Function(SendPort sendPort);

class MockDiscoveryEngineWorker extends DiscoveryEngineWorker {
  final EngineEvent initResponse;
  final EngineEvent configurationChangedResponse;
  final EngineEvent restoreFeedRequestedResponse;
  final EngineEvent nextFeedBatchRequestedResponse;
  final EngineEvent feedDocumentsClosedResponse;
  final EngineEvent excludedSourceAddedResponse;
  final EngineEvent excludedSourceRemovedResponse;
  final EngineEvent excludedSourcesListRequestedResponse;
  final EngineEvent trustedSourceAddedResponse;
  final EngineEvent trustedSourceRemovedResponse;
  final EngineEvent trustedSourcesListRequestedResponse;
  final EngineEvent availableSourcesListRequestedResponse;
  final EngineEvent userReactionChangedResponse;
  final EngineEvent documentTimeLoggedResponse;
  final EngineEvent activeSearchRequestedResponse;
  final EngineEvent nextActiveSearchBatchRequestedResponse;
  final EngineEvent restoreActiveSearchResponse;
  final EngineEvent activeSearchClosedResponse;
  final EngineEvent activeSearchTermRequestedResponse;
  final EngineEvent deepSearchRequestedResponse;
  final EngineEvent trendingTopicsRequestedResponse;
  final EngineEvent resetAiRequestedResponse;
  final EngineEvent setSourcesRequestedResponse;

  MockDiscoveryEngineWorker(
    Object initialMessage, {
    this.initResponse = const EngineEvent.clientEventSucceeded(),
    this.configurationChangedResponse =
        const EngineEvent.clientEventSucceeded(),
    this.restoreFeedRequestedResponse =
        const EngineEvent.restoreFeedSucceeded([]),
    this.nextFeedBatchRequestedResponse =
        const EngineEvent.nextFeedBatchRequestSucceeded([]),
    this.feedDocumentsClosedResponse = const EngineEvent.clientEventSucceeded(),
    this.userReactionChangedResponse = const EngineEvent.clientEventSucceeded(),
    this.documentTimeLoggedResponse = const EngineEvent.clientEventSucceeded(),
    EngineEvent? activeSearchRequestedResponse,
    EngineEvent? nextActiveSearchBatchRequestedResponse,
    EngineEvent? restoreActiveSearchResponse,
    this.activeSearchClosedResponse = const EngineEvent.clientEventSucceeded(),
    this.activeSearchTermRequestedResponse =
        const EngineEvent.activeSearchTermRequestSucceeded(queryTerm),
    this.deepSearchRequestedResponse =
        const EngineEvent.deepSearchRequestSucceeded([]),
    this.excludedSourceAddedResponse = const EngineEvent.clientEventSucceeded(),
    this.excludedSourceRemovedResponse =
        const EngineEvent.clientEventSucceeded(),
    EngineEvent? excludedSourcesListRequestedResponse,
    this.trustedSourceAddedResponse = const EngineEvent.clientEventSucceeded(),
    this.trustedSourceRemovedResponse =
        const EngineEvent.clientEventSucceeded(),
    EngineEvent? trustedSourcesListRequestedResponse,
    this.trendingTopicsRequestedResponse =
        const EngineEvent.trendingTopicsRequestSucceeded([]),
    EngineEvent? availableSourcesListRequestedResponse,
    this.resetAiRequestedResponse = const EngineEvent.clientEventSucceeded(),
    EngineEvent? setSourcesRequestedResponse,
  })  : activeSearchRequestedResponse =
            EngineEvent.activeSearchRequestSucceeded(
          mockActiveSearch.toApiRepr(),
          [],
        ),
        nextActiveSearchBatchRequestedResponse =
            EngineEvent.nextActiveSearchBatchRequestSucceeded(
          mockActiveSearch.toApiRepr(),
          [],
        ),
        restoreActiveSearchResponse = EngineEvent.restoreActiveSearchSucceeded(
          mockActiveSearch.toApiRepr(),
          [],
        ),
        setSourcesRequestedResponse = setSourcesRequestedResponse ??
            EngineEvent.setSourcesRequestSucceeded(
              trustedSources: {Source('trusted.example.com')},
              excludedSources: {Source('excluded.example.com')},
            ),
        excludedSourcesListRequestedResponse =
            excludedSourcesListRequestedResponse ??
                EngineEvent.excludedSourcesListRequestSucceeded(
                  {Source('example.com')},
                ),
        trustedSourcesListRequestedResponse =
            trustedSourcesListRequestedResponse ??
                EngineEvent.trustedSourcesListRequestSucceeded(
                  {Source('example.com')},
                ),
        availableSourcesListRequestedResponse =
            availableSourcesListRequestedResponse ??
                EngineEvent.availableSourcesListRequestSucceeded(
                  mockedAvailableSources.list,
                ),
        super(initialMessage);

  @override
  Future<void> onMessage(request) async {
    final response = request.payload.map<EngineEvent>(
      init: (_) => initResponse,
      configurationChanged: (_) => configurationChangedResponse,
      restoreFeedRequested: (_) => restoreFeedRequestedResponse,
      nextFeedBatchRequested: (_) => nextFeedBatchRequestedResponse,
      feedDocumentsClosed: (_) => feedDocumentsClosedResponse,
      excludedSourceAdded: (_) => excludedSourceAddedResponse,
      excludedSourceRemoved: (_) => excludedSourceRemovedResponse,
      excludedSourcesListRequested: (_) => excludedSourcesListRequestedResponse,
      trustedSourceAdded: (_) => trustedSourceAddedResponse,
      trustedSourceRemoved: (_) => trustedSourceRemovedResponse,
      trustedSourcesListRequested: (_) => trustedSourcesListRequestedResponse,
      availableSourcesListRequested: (_) =>
          availableSourcesListRequestedResponse,
      userReactionChanged: (_) => userReactionChangedResponse,
      documentTimeSpent: (_) => documentTimeLoggedResponse,
      activeSearchRequested: (_) => activeSearchRequestedResponse,
      nextActiveSearchBatchRequested: (_) =>
          nextActiveSearchBatchRequestedResponse,
      restoreActiveSearchRequested: (_) => restoreActiveSearchResponse,
      activeSearchClosed: (_) => activeSearchClosedResponse,
      activeSearchTermRequested: (_) => activeSearchTermRequestedResponse,
      deepSearchRequested: (_) => deepSearchRequestedResponse,
      trendingTopicsRequested: (_) => trendingTopicsRequestedResponse,
      resetAi: (_) => resetAiRequestedResponse,
      setSourcesRequested: (_) => setSourcesRequestedResponse,
    );
    return send(response, request.sender);
  }

  static void main(Object initialMessage) =>
      MockDiscoveryEngineWorker(initialMessage);
}

Future<DiscoveryEngine> createEngineWithEntryPoint(Object entryPoint) =>
    DiscoveryEngine.init(configuration: mockConfig, entryPoint: entryPoint);

void withSuccessResponse(Object initialMessage) =>
    MockDiscoveryEngineWorker(initialMessage);

final mockConfig = Configuration(
  apiKey: '**********',
  apiBaseUrl: 'https://api.example.com',
  assetsUrl: 'https://ai-assets.example.com',
  maxItemsPerFeedBatch: 50,
  maxItemsPerSearchBatch: 20,
  applicationDirectoryPath: './',
  feedMarkets: {const FeedMarket(langCode: 'de', countryCode: 'DE')},
  manifest: Manifest.fromJson(goodJson),
  headlinesProviderPath: '/newscatcher/v1/latest-headlines',
  newsProviderPath: '/newscatcher/v1/search-news',
);

final mockNewsResource = NewsResource(
  title: 'Example',
  snippet: 'snippet',
  url: Uri.parse('https://example.com'),
  sourceDomain: Source('example.com'),
  datePublished: DateTime.utc(2022, 01, 01),
  image: Uri.parse('http://thumbnail.example.com'),
  rank: 10,
  score: 0.1,
  country: 'US',
  language: 'en',
  topic: 'news',
);

List<DocumentWithActiveData> mockDocuments(
  StackId stackId,
  bool isSearched,
) =>
    [
      DocumentWithActiveData(
        Document(
          documentId: DocumentId(),
          stackId: stackId,
          batchIndex: 0,
          resource: mockNewsResource,
          isSearched: isSearched,
        ),
        ActiveDocumentData(Embedding.fromList([1, 3])),
      ),
      DocumentWithActiveData(
        Document(
          documentId: DocumentId(),
          stackId: stackId,
          batchIndex: 1,
          resource: mockNewsResource,
          isSearched: isSearched,
        ),
        ActiveDocumentData(Embedding.fromList([0])),
      ),
    ];

const queryTerm = 'example';

final mockActiveSearch = ActiveSearch(
  searchBy: SearchBy.query,
  searchTerm: queryTerm,
  requestedPageNb: 1,
  pageSize: 20,
);
