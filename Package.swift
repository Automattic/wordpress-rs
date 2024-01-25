// swift-tools-version:5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.
// Swift Package: WordpressApi

import PackageDescription

let package = Package(
    name: "wordpress",
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15),
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
                .target(name: "wordpress-api-wrapper"),
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
        .binaryTarget(name: "libwordpressFFI", path: "target/libwordpressFFI.xcframework"),
        .testTarget(
            name: "wordpress-api-tests",
            dependencies: [
                .target(name: "wordpress-api")
            ],
            path: "native/swift/Tests/wordpress-api"
        )
    ]
)
