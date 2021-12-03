import 'dart:convert' show Converter;
import 'package:xayn_discovery_engine/src/api/api.dart'
    show ClientEvent, EngineEvent;
import 'package:xayn_discovery_engine/src/api/codecs/json_codecs.dart'
    show JsonToOneshotRequestConverter, EngineEventToJsonConverter;
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart';
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show Worker, OneshotRequest;

class DiscoveryEngineWorker extends Worker<ClientEvent, EngineEvent> {
  final _requestConverter = JsonToOneshotRequestConverter();
  final _responseConverter = EngineEventToJsonConverter();

  @override
  Converter<Map<String, Object>, OneshotRequest<ClientEvent>>
      get requestConverter => _requestConverter;

  @override
  Converter<EngineEvent, Map<String, Object>> get responseConverter =>
      _responseConverter;

  DiscoveryEngineWorker(Object message) : super(message);

  @override
  void onError(Object error) {
    // TODO: handle errors
  }

  @override
  Future<void> onMessage(request) async {
    final clientEvent = request.payload;
    // This is just initial handler to respond with some events
    //
    // TODO: replace with proper handler
    // Events are grouped
    // if (clientEvent is SystemClientEvent) {
    //   // pass the event to dedicated manager
    // } else if (clientEvent is FeedClientEvent) {
    //   // pass the event to DocumentManager
    // } else if (clientEvent is DocumentClientEvent) {
    //   // pass the event to DocumentManager
    // } else {
    //   // handle wrong event type???
    // }
    final response = await clientEvent.maybeWhen(
      init: (configuration) async {
        return const EngineEvent.clientEventSucceeded();
      },
      resetEngine: () async {
        return const EngineEvent.clientEventSucceeded();
      },
      configurationChanged: (feedMarket, maxItemsPerFeedBatch) async {
        return const EngineEvent.clientEventSucceeded();
      },
      feedRequested: () async {
        return const EngineEvent.feedRequestSucceeded([]);
      },
      nextFeedBatchRequested: () async {
        return const EngineEvent.nextFeedBatchRequestSucceeded([]);
      },
      feedDocumentsClosed: (documentIds) async {
        return const EngineEvent.clientEventSucceeded();
      },
      documentFeedbackChanged: (documentId, feedback) async {
        return const EngineEvent.clientEventSucceeded();
      },
      documentStatusChanged: (documentId, status) async {
        return const EngineEvent.clientEventSucceeded();
      },
      documentClosed: (documentId) async {
        return const EngineEvent.clientEventSucceeded();
      },
      orElse: () async {
        return const EngineEvent.engineExceptionRaised(
          EngineExceptionReason.wrongEventRequested,
        );
      },
    );

    send(response, request.sender);
  }
}

/// This method acts as an entry point:
/// - for Isolate.spawn on native platform
/// - for the compiled web worker file
void main(Object message) => DiscoveryEngineWorker(message);
