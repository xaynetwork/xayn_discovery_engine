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
import 'package:xayn_discovery_engine/src/discovery_engine_base.dart'
    show DiscoveryEngine;
import 'package:xayn_discovery_engine/src/discovery_engine_worker.dart'
    show DiscoveryEngineWorker;
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResource;

typedef EntryPoint = void Function(SendPort sendPort);

class MockDiscoveryEngineWorker extends DiscoveryEngineWorker {
  final EngineEvent initResponse;
  final EngineEvent resetEngineResponse;
  final EngineEvent configurationChangedResponse;
  final EngineEvent feedRequestedResponse;
  final EngineEvent nextFeedBatchRequestedResponse;
  final EngineEvent feedDocumentsClosedResponse;
  final EngineEvent documentFeedbackChangedResponse;
  final EngineEvent documentTimeLoggedResponse;

  MockDiscoveryEngineWorker(
    Object initialMessage, {
    this.initResponse = const EngineEvent.clientEventSucceeded(),
    this.resetEngineResponse = const EngineEvent.clientEventSucceeded(),
    this.configurationChangedResponse =
        const EngineEvent.clientEventSucceeded(),
    this.feedRequestedResponse = const EngineEvent.feedRequestSucceeded([]),
    this.nextFeedBatchRequestedResponse =
        const EngineEvent.nextFeedBatchRequestSucceeded([]),
    this.feedDocumentsClosedResponse = const EngineEvent.clientEventSucceeded(),
    this.documentFeedbackChangedResponse =
        const EngineEvent.clientEventSucceeded(),
    this.documentTimeLoggedResponse = const EngineEvent.clientEventSucceeded(),
  }) : super(initialMessage);

  @override
  Future<void> onMessage(request) async {
    final response = request.payload.map<EngineEvent>(
      init: (_) => initResponse,
      resetEngine: (_) => resetEngineResponse,
      configurationChanged: (_) => configurationChangedResponse,
      feedRequested: (_) => feedRequestedResponse,
      nextFeedBatchRequested: (_) => nextFeedBatchRequestedResponse,
      feedDocumentsClosed: (_) => feedDocumentsClosedResponse,
      documentFeedbackChanged: (_) => documentFeedbackChangedResponse,
      documentTimeSpent: (_) => documentTimeLoggedResponse,
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
  apiBaseUrl: 'https://example-api.dev',
  assetsUrl: 'https://ai-assets.dev',
  maxItemsPerFeedBatch: 50,
  applicationDirectoryPath: './',
  feedMarkets: {const FeedMarket(countryCode: 'DE', langCode: 'de')},
);

final mockNewsResource = NewsResource(
  title: 'Example',
  snippet: 'snippet',
  url: Uri.parse('https://domain.com'),
  displayUrl: Uri.parse('domain.com'),
  datePublished: DateTime.utc(2022, 01, 01),
  thumbnail: Uri.parse('http://thumbnail.domain.com'),
  rank: 10,
  score: 0.1,
  country: 'EN',
  language: 'en',
  topic: 'news',
);
