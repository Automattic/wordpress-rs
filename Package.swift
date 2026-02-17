// swift-tools-version: 6.2

import Foundation
import PackageDescription

let libwordpressFFIVersion: WordPressRSVersion = .release(version: "alpha-20260216", checksum: "f48184673018895e30860eb55b3a23e44e90607f5ebe68110071ea8dfe73c0a6")

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

// MARK: - Enable local development toolings

let localDevelopment = libwordpressFFIVersion.isLocal

if localDevelopment {
    try enableSwiftLint()
}

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

// Add SwiftLint to the package so that we can see linting issues directly from Xcode.
@MainActor
func enableSwiftLint() throws {
#if os(macOS)
    let filePath = URL(string:"./.swiftlint.yml", relativeTo: URL(filePath: #filePath))!
    let version = try String(contentsOf: filePath, encoding: .utf8)
        .split(separator: "\n")
        .first(where: { $0.starts(with: "swiftlint_version") })?
        .split(separator: ":")
        .last?
        .trimmingCharacters(in: .whitespaces)
    guard let version else {
        fatalError("Can't find swiftlint_version in .swiftlint.yml")
    }

    package.dependencies.append(.package(url: "https://github.com/realm/SwiftLint", exact: .init(version)!))
#endif
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
