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

let supportBackgroundURLSession: SwiftSetting = .define("WP_SUPPORT_BACKGROUND_URL_SESSION", .when(platforms: [.macOS, .iOS, .tvOS, .watchOS]))

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
    dependencies: [
        .package(url: "https://github.com/apple/swift-crypto", .upToNextMajor(from: "3.3.0")),
        .package(url: "https://github.com/swhitty/FlyingFox", exact: "0.12.2"),
    ],
    targets: [
        .target(
            name: "wordpress-api",
            dependencies: [
                .target(name: "wordpress-api-wrapper")
            ],
            path: "native/swift/Sources/wordpress-api",
            swiftSettings: [
                supportBackgroundURLSession
            ]
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
                .target(name: "libwordpressFFI"),
                .product(name: "Crypto", package: "swift-crypto"),
                "FlyingFox",
            ],
            path: "native/swift/Tests/wordpress-api",
            swiftSettings: [
                supportBackgroundURLSession
            ]
        )
    ]
)
