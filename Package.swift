// swift-tools-version:5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.
// Swift Package: WordpressApi

import PackageDescription

#if os(Linux)
let libwordpressFFI: Target = .systemLibrary(
        name: "libwordpressFFI",
        path: "target/swift-bindings/libwordpressFFI-linux/"
    )
#elseif os(macOS)
let libwordpressFFI: Target = .binaryTarget(name: "libwordpressFFI", path: "target/libwordpressFFI.xcframework")
#endif

let package = Package(
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
            path: "native/swift/Sources/wordpress-api"
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
