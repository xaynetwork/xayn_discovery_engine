import 'package:xayn_discovery_engine/src/api/api.dart'
    show ClientEventGroups, ClientEventSucceeded, EngineEventGroups, Init;
import 'package:xayn_discovery_engine/src/discovery_engine_manager.dart'
    show DiscoveryEngineManager;
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;

/// This class exposes Xayn Discovery Engine API to the clients.
class DiscoveryEngine {
  final DiscoveryEngineManager _manager;

  DiscoveryEngine._(this._manager);

  /// Stream of [EngineEventGroups] coming back from a discovery engine worker.
  Stream<EngineEventGroups> get engineEvents => _manager.responses;

  static Future<DiscoveryEngine> init({
    required Configuration configuration,
  }) async {
    try {
      final manager = await DiscoveryEngineManager.create();
      final initEvent = ClientEventGroups.system(event: Init(configuration));
      final response = await manager.send(initEvent);

      final wasInitSuccessful = response.maybeWhen(
        system: (event) => event is ClientEventSucceeded,
        orElse: () => false,
      );

      if (!wasInitSuccessful) throw StateError('something went very wrong');

      return DiscoveryEngine._(manager);
    } catch (e) {
      //
      throw StateError('something went very wrong');
    }
  }
}
