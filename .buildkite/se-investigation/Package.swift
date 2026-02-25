// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "SEInvestigation",
    platforms: [.macOS(.v14)],
    targets: [
        .testTarget(name: "SEInvestigationTests")
    ]
)
