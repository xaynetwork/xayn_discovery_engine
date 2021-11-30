import 'dart:convert' show Converter;

import 'package:xayn_discovery_engine/src/api/api.dart'
    show ClientEventGroups, ClientEventSucceeded, EngineEventGroups;
import 'package:xayn_discovery_engine/src/api/codecs/json_codecs.dart'
    show JsonToOneshotRequestConverter, EngineEventGroupsToJsonConverter;
import 'package:xayn_discovery_engine/src/api/events/engine_events/system_events.dart';
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show Worker, OneshotRequest;

class DiscoveryEngineWorker
    extends Worker<ClientEventGroups, EngineEventGroups> {
  final _requestConverter = JsonToOneshotRequestConverter();
  final _responseConverter = EngineEventGroupsToJsonConverter();

  @override
  Converter<Map<String, dynamic>, OneshotRequest<ClientEventGroups>>
      get requestConverter => _requestConverter;

  @override
  Converter<EngineEventGroups, Map<String, dynamic>> get responseConverter =>
      _responseConverter;

  DiscoveryEngineWorker(Object message) : super(message);

  @override
  void onError(Object error) {
    // send('$error');
  }

  @override
  void onMessage(request) {
    final response = request.payload.maybeWhen(
      system: (event) {
        event.maybeWhen(
          init: (configuration) {
            return EngineEventGroups.system(event: ClientEventSucceeded());
          },
          orElse: () {
            return EngineEventGroups.system(
              event: EngineExceptionRaised(
                EngineExceptionReason.noInitReceived,
              ),
            );
          },
        );
      },
      orElse: () {
        return EngineEventGroups.system(
          event: EngineExceptionRaised(
            EngineExceptionReason.noInitReceived,
          ),
        );
      },
    );

    send(response!, request.sender);
  }
}

/// This method acts as an entry point:
/// - for Isolate.spawn on native platform
/// - for the compiled web worker file
void main(Object message) => DiscoveryEngineWorker(message);
