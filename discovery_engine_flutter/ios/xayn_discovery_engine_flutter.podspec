#
# To learn more about a Podspec see http://guides.cocoapods.org/syntax/podspec.html.
# Run `pod lib lint xayn_discovery_engine_flutter.podspec` to validate before publishing.
#
Pod::Spec.new do |s|
  s.name             = 'xayn_discovery_engine_flutter'
  s.version          = '0.0.1'
  s.summary          = 'Xayn Discovery Engine flutter plugin project.'
  s.description      = <<-DESC
Xayn Discovery Engine flutter plugin project.
                       DESC
  s.homepage         = 'http://xayn.com'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'Xayn' => 'engineering@xaynet.dev' }
  s.source           = { :path => '.' }
  s.source_files = 'Classes/**/*'
  s.dependency 'Flutter'
  s.platform = :ios, '9.0'

  # Flutter.framework does not contain a i386 slice.
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES', 'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386' }
  s.swift_version = '5.0'
  # TODO: uncomment xcconfig line when binaries are included in the release ci
  # Forces loading the binaries
  # s.xcconfig = { 'OTHER_LDFLAGS' => '-force_load "${PODS_ROOT}/../.symlinks/plugins/xayn_discovery_engine_flutter/ios/libxayn_discovery_engine_bindings_x86_64-apple-ios.a" -force_load "${PODS_ROOT}/../.symlinks/plugins/xayn_discovery_engine_flutter/ios/libxayn_discovery_engine_bindings_aarch64-apple-ios.a"'}
end
