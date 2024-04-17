// swift-tools-version:5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.
// Swift Package: WordpressApi

import Foundation
import PackageDescription

let isCI = ProcessInfo.processInfo.environment["CI"] == "true"

#if os(Linux)
let libwordpressFFI: Target = .systemLibrary(
        name: "libwordpressFFI",
        path: "target/swift-bindings/libwordpressFFI-linux/"
    )
#elseif os(macOS)
let libwordpressFFI: Target = .binaryTarget(name: "libwordpressFFI", path: "target/libwordpressFFI.xcframework")
#endif

#if os(macOS)
let e2eTestsEnabled = !isCI
#elseif os(Linux)
let e2eTestsEnabled = true
#else
let e2eTestsEnabled = false
#endif

var additionalTestTargets = [Target]()

if e2eTestsEnabled {
    additionalTestTargets.append(.testTarget(
        name: "End2EndTests",
        dependencies: [
            .target(name: "wordpress-api"),
            .target(name: "libwordpressFFI")
        ],
        path: "native/swift/Tests/End2End"
    ))
}

let package = Package(
    name: "wordpress",
    platforms: [
        .iOS(.v13),
        .macOS(.v11),
        .tvOS(.v13),
        .watchOS(.v8)
    ],
    products: [
        .library(
            name: "wordpress-api",
            targets: ["wordpress-api"]
        )
    ],
    dependencies: [],
    targets: [
        .target(
            name: "wordpress-api",
            dependencies: [
                .target(name: "wordpress-api-wrapper")
            ],
            path: "native/swift/Sources/wordpress-api"
        ),
        .target(
            name: "wordpress-api-wrapper",
            dependencies: [
                .target(name: "libwordpressFFI")
            ],
            path: "native/swift/Sources/wordpress-api-wrapper",
            exclude: [
                "README.md"
            ]
        ),
        libwordpressFFI,
        .testTarget(
            name: "wordpress-api-tests",
            dependencies: [
                .target(name: "wordpress-api"),
                .target(name: "libwordpressFFI")
            ],
            path: "native/swift/Tests/wordpress-api"
        )
    ] + additionalTestTargets
)
