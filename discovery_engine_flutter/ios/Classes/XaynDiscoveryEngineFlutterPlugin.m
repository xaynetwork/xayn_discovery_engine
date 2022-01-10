#import "XaynDiscoveryEngineFlutterPlugin.h"
#if __has_include(<xayn_discovery_engine_flutter/xayn_discovery_engine_flutter-Swift.h>)
#import <xayn_discovery_engine_flutter/xayn_discovery_engine_flutter-Swift.h>
#else
// Support project import fallback if the generated compatibility header
// is not copied when this plugin is created as a library.
// https://forums.swift.org/t/swift-static-libraries-dont-copy-generated-objective-c-header/19816
#import "xayn_discovery_engine_flutter-Swift.h"
#endif

@implementation XaynDiscoveryEngineFlutterPlugin
+ (void)registerWithRegistrar:(NSObject<FlutterPluginRegistrar>*)registrar {
  [SwiftXaynDiscoveryEngineFlutterPlugin registerWithRegistrar:registrar];
}
@end
