// swift-tools-version:5.10

import Foundation
import PackageDescription

let libwordpressFFIVersion: WordPressRSVersion = .local

#if os(Linux)
let libwordpressFFI: Target = .systemLibrary(
        name: "libwordpressFFI",
        path: "target/swift-bindings/libwordpressFFI-linux/"
    )
#elseif os(macOS)
let libwordpressFFI: Target = libwordpressFFIVersion.target
#endif

var package = Package(
    name: "WordPress",
    platforms: [
        .iOS(.v13),
        .macOS(.v11),
        .tvOS(.v13),
        .watchOS(.v8)
    ],
    products: [
        .library(
            name: "WordPressAPI",
            targets: ["WordPressAPI"]
        )
    ],
    dependencies: [],
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
            ]
        ),
        libwordpressFFI,
        .testTarget(
            name: "WordPressAPITests",
            dependencies: [
                .target(name: "WordPressAPI"),
                .target(name: libwordpressFFI.name)
            ],
            path: "native/swift/Tests/wordpress-api"
        )
    ]
)

// MARK: - Enable local development toolings

#if os(macOS)
let localDevelopment = libwordpressFFIVersion.isLocal
#else
let localDevelopment = false
#endif

if localDevelopment {
    try enableSwiftLint()
    addCodegenTarget()
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
                url: "https://github.com/Automattic/wordpress-rs/releases/download/\(version)/libwordpressFFI.xcframework.zip",
                checksum: checksum
            )
        }
    }
}

// Add SwiftLint to the package so that we can see linting issues directly from Xcode.
func enableSwiftLint() throws {
    let version = try String(contentsOf: URL(string:"./.swiftlint.yml", relativeTo: URL(filePath: #filePath))!)
        .split(separator: "\n")
        .first(where: { $0.starts(with: "swiftlint_version") })?
        .split(separator: ":")
        .last?
        .trimmingCharacters(in: .whitespaces)
    guard let version else {
        fatalError("Can't find swiftlint_version in .swiftlint.yml")
    }

    package.dependencies.append(.package(url: "https://github.com/realm/SwiftLint", exact: .init(version)!))

    var platforms = package.platforms ?? []
    if let mac = platforms.firstIndex(where: { $0 == .macOS(.v11) }) {
        platforms.remove(at: mac)
        platforms.append(.macOS(.v12))
    }
    package.platforms = platforms

    if let target = package.targets.first(where: { $0.name == "WordPressAPI" }) {
        target.plugins = (target.plugins ?? []) + [.plugin(name: "SwiftLintBuildToolPlugin", package: "SwiftLint")]
    }
}

func addCodegenTarget() {
    package.dependencies.append(.package(url: "https://github.com/apple/swift-syntax.git", from: "510.0.0"))

    let target = Target.executableTarget(
        name: "codegen",
        dependencies: [
            .product(name: "SwiftSyntax", package: "swift-syntax"),
            .product(name: "SwiftParser", package: "swift-syntax"),
            .product(name: "SwiftSyntaxBuilder", package: "swift-syntax"),
        ],
        path: "native/swift/Sources/codegen"
    )
    package.targets.append(target)
}
