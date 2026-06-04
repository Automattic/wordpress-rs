// swift-tools-version: 6.2

import Foundation
import PackageDescription

let libwordpressFFIVersion: WordPressRSVersion = .release(version: "pr-builds/1367", checksum: "4840967567307007f04a0bd12664fc683a445ad7595ec182c98b54da30360d42")

#if os(Linux)
let libwordpressFFI: Target = .systemLibrary(
        name: "libwordpressFFI",
        path: "target/release/libwordpressFFI-linux/"
    )
#elseif os(macOS)
let libwordpressFFI: Target = libwordpressFFIVersion.target
#endif

var package = Package(
    name: "WordPressAPI",
    platforms: [
        .iOS(.v16),
        .macOS(.v13),
        .tvOS(.v16),
        .watchOS(.v9)
    ],
    products: [
        .library(
            name: "WordPressAPI",
            targets: ["WordPressAPI"]
        ),
        .library(
            name: "WordPressApiCache",
            targets: ["WordPressApiCache"]
        )
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-docc-plugin", from: "1.0.0"),
    ],
    targets: [
        .target(
            name: "WordPressAPI",
            dependencies: [
                .target(name: "WordPressAPIInternal"),
                .target(name: "WordPressApiCache")
            ],
            path: "native/swift/Sources/wordpress-api",
            swiftSettings: [
                .enableExperimentalFeature("StrictConcurrency"),
                .define("PROGRESS_REPORTING_ENABLED", .when(platforms: [.iOS, .macOS, .tvOS, .watchOS]))
            ]
        ),
        .target(
            name: "WordPressAPIInternal",
            dependencies: [
                .target(name: libwordpressFFI.name)
            ],
            path: "native/swift/Sources/wordpress-api-wrapper",
            exclude: [
                "README.md"
            ],
            swiftSettings: [
                .swiftLanguageMode(.v5)
            ]
        ),
        .target(
            name: "WordPressApiCache",
            dependencies: [
                .target(name: "WordPressAPIInternal")
            ],
            path: "native/swift/Sources/wordpress-api-cache"
        ),
        libwordpressFFI,
        .testTarget(
            name: "WordPressAPITests",
            dependencies: [
                .target(name: "WordPressAPI"),
                .target(name: "WordPressApiCache"),
                .target(name: libwordpressFFI.name)
            ],
            path: "native/swift/Tests/wordpress-api",
            resources: [.copy("../../../../test-data/integration-test-responses/")],
            swiftSettings: [
                .define("PROGRESS_REPORTING_ENABLED", .when(platforms: [.iOS, .macOS, .tvOS, .watchOS]))
            ]
        ),
        .testTarget(
            name: "WordPressApiCacheTests",
            dependencies: [
                .target(name: "WordPressApiCache"),
                .target(name: "WordPressAPIInternal"),
                .target(name: "WordPressAPI")
            ],
            path: "native/swift/Tests/wordpress-api-cache"
        ),
        .testTarget(
            name: "WordPressApiCompatibilityTests",
            dependencies: [
                .target(name: "WordPressAPI"),
            ],
            path: "native/swift/Tests/api-compatibility"
        )
    ].addingIntegrationTests()
)

// MARK: - Helpers

enum WordPressRSVersion {
    case local
    case release(version: String, checksum: String)

    var isLocal: Bool {
        if case .local = self {
            return true
        }
        return false
    }

    var target: Target {
        switch libwordpressFFIVersion {
        case .local:
            return .binaryTarget(name: "libwordpressFFI", path: "target/libwordpressFFI.xcframework")
        case let .release(version, checksum):
            return .binaryTarget(
                name: "libwordpressFFI",
                url: "https://cdn.a8c-ci.services/wordpress-rs/\(version)/libwordpressFFI.xcframework.zip",
                checksum: checksum
            )
        }
    }
}


extension Array where Element == Target {

    // Run `make test-server` before running integration tests.
    func addingIntegrationTests() -> Self {
        var enabled = false

        if Context.environment["BUILDKITE"] != nil {
            // When running on CI, only enable integration tests on Linux, since macOS CI agent does not have docker.
            #if os(Linux)
            enabled = true
            #endif
        } else {
            // Enable integration tests during local development, since we can easily install docker env on our macOS.
            enabled = true
        }

        if enabled {
            return self + [.testTarget(
                name: "IntegrationTests",
                dependencies: [
                    .target(name: "WordPressAPI"),
                ],
                path: "native/swift/Tests/integration-tests",
                resources: [.copy("../../../../test-data/")]
            )]
        } else {
            return self
        }
    }
}
