Pod::Spec.new do |spec|
  spec.name         = "WordPressAPI"
  spec.version      = "0.0.1"
  spec.summary      = "WordPressAPI."
  spec.description  = "WordPress API in Swift."

  spec.homepage     = "https://github.com/automattic/wordpress-rs"
  spec.license      = "MIT"
  spec.author             = { "Tony Li" => "tony.li@automattic.com" }

  spec.ios.deployment_target = '13.0'
  spec.osx.deployment_target = '11.0'

  # zip -r swift-source-archive.zip native/swift target/libwordpressFFI.xcframework
  spec.source       = { :http => "http://s3.com/WordPressAPI.zip" }

  spec.swift_version = '5.10'
  spec.source_files  = 'native/swift/Sources/**/*.{swift}'
  spec.vendored_frameworks = 'target/libwordpressFFI.xcframework'

  spec.pod_target_xcconfig = {
    'SWIFT_PACKAGE_NAME' => 'WordPressAPI'
  }

  spec.test_spec 'Tests' do |test_spec|
    test_spec.source_files = 'native/swift/Tests/**/*.{swift}'
  end
end
