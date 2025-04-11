// swift-tools-version: 6.1

import Foundation
import PackageDescription

let libwordpressFFIVersion: WordPressRSVersion = .release(version: "alpha-20250411", checksum: "b01d1900fd63f154006944c4728492678db04447e9284cd1a696cb3ffec9c2c4")

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
        .iOS(.v15),
        .macOS(.v12),
        .tvOS(.v16),
        .watchOS(.v8)
    ],
    products: [
        .library(
            name: "WordPressAPI",
            targets: ["WordPressAPI"]
        )
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-docc-plugin", from: "1.0.0"),
    ],
    targets: [
        .target(
            name: "WordPressAPI",
            dependencies: [
                .target(name: "WordPressAPIInternal")
            ],
            path: "native/swift/Sources/wordpress-api",
            swiftSettings: [
                .enableExperimentalFeature("StrictConcurrency"),
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
        libwordpressFFI,
        .testTarget(
            name: "WordPressAPITests",
            dependencies: [
                .target(name: "WordPressAPI"),
                .target(name: libwordpressFFI.name)
            ],
            path: "native/swift/Tests/wordpress-api",
            resources: [.copy("Resources/Responses/")]
        )
    ]
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
