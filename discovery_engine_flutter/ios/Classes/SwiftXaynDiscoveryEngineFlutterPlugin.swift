import Flutter
import UIKit

public class SwiftXaynDiscoveryEngineFlutterPlugin: NSObject, FlutterPlugin {
  public static func register(with registrar: FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(name: "xayn_discovery_engine_flutter", binaryMessenger: registrar.messenger())
    let instance = SwiftXaynDiscoveryEngineFlutterPlugin()
    registrar.addMethodCallDelegate(instance, channel: channel)
  }

  public func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    result("iOS " + UIDevice.current.systemVersion)
  }
}
