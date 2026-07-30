// swift-tools-version: 6.1

import PackageDescription

let package = Package(
    name: "CodexGaugeNative",
    platforms: [
        .macOS(.v15),
    ],
    products: [
        .library(name: "CodexGaugeCore", targets: ["CodexGaugeCore"]),
    ],
    targets: [
        .target(
            name: "CodexGaugeCore",
            path: "CodexGaugeNative/Core"
        ),
        .testTarget(
            name: "CodexGaugeCoreTests",
            dependencies: ["CodexGaugeCore"],
            path: "Tests/CodexGaugeCoreTests"
        ),
    ]
)

