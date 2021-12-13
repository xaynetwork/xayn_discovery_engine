import 'dart:isolate' show SendPort;

import 'package:xayn_discovery_engine/src/api/api.dart'
    show Configuration, EngineEvent;
import 'package:xayn_discovery_engine/src/discovery_engine_base.dart'
    show DiscoveryEngine;
import 'package:xayn_discovery_engine/src/discovery_engine_worker.dart'
    show DiscoveryEngineWorker;

typedef EntryPoint = void Function(SendPort sendPort);

class MockDiscoveryEngineWorker extends DiscoveryEngineWorker {
  final EngineEvent initResponse;
  final EngineEvent resetEngineResponse;
  final EngineEvent configurationChangedResponse;
  final EngineEvent feedRequestedResponse;
  final EngineEvent nextFeedBatchRequestedResponse;
  final EngineEvent feedDocumentsClosedResponse;
  final EngineEvent documentStatusChangedResponse;
  final EngineEvent documentFeedbackChangedResponse;
  final EngineEvent documentClosedResponse;

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
    this.documentStatusChangedResponse =
        const EngineEvent.clientEventSucceeded(),
    this.documentFeedbackChangedResponse =
        const EngineEvent.clientEventSucceeded(),
    this.documentClosedResponse = const EngineEvent.clientEventSucceeded(),
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
      documentStatusChanged: (_) => documentStatusChangedResponse,
      documentFeedbackChanged: (_) => documentFeedbackChangedResponse,
      documentClosed: (_) => documentClosedResponse,
    );
    return send(response, request.sender);
  }

  static void main(Object initialMessage) =>
      MockDiscoveryEngineWorker(initialMessage);
}

Future<DiscoveryEngine> createEngineWithEntryPoint(Object entryPoint) =>
    DiscoveryEngine.init(configuration: mockConfig, entryPoint: entryPoint);

const mockConfig = Configuration(
  apiKey: '**********',
  apiBaseUrl: 'https://example-api.dev',
  feedMarket: 'de-DE',
  maxItemsPerFeedBatch: 50,
  applicationDirectoryPath: './',
);
