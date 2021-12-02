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
  Converter<Map<String, dynamic>, OneshotRequest<ClientEvent>>
      get requestConverter => _requestConverter;

  @override
  Converter<EngineEvent, Map<String, dynamic>> get responseConverter =>
      _responseConverter;

  DiscoveryEngineWorker(Object message) : super(message);

  @override
  void onError(Object error) {
    // TODO: handle errors
  }

  @override
  void onMessage(request) {
    // TODO: it should be replaced by a message handler
    final response = request.payload.maybeWhen(
      init: (configuration) => const EngineEvent.clientEventSucceeded(),
      orElse: () => const EngineEvent.engineExceptionRaised(
        EngineExceptionReason.genericError,
      ),
    );

    send(response, request.sender);
  }
}

/// This method acts as an entry point:
/// - for Isolate.spawn on native platform
/// - for the compiled web worker file
void main(Object message) => DiscoveryEngineWorker(message);
