// swift-tools-version:5.3

import Foundation
import PackageDescription

var sources = ["src/parser.c"]
if FileManager.default.fileExists(atPath: "src/scanner.c") {
    sources.append("src/scanner.c")
}

let package = Package(
    name: "TreeSitterQuipu",
    products: [
        .library(name: "TreeSitterQuipu", targets: ["TreeSitterQuipu"]),
    ],
    dependencies: [
        .package(name: "SwiftTreeSitter", url: "https://github.com/tree-sitter/swift-tree-sitter", from: "0.9.0"),
    ],
    targets: [
        .target(
            name: "TreeSitterQuipu",
            dependencies: [],
            path: ".",
            sources: sources,
            resources: [
                .copy("queries")
            ],
            publicHeadersPath: "bindings/swift",
            cSettings: [.headerSearchPath("src")]
        ),
        .testTarget(
            name: "TreeSitterQuipuTests",
            dependencies: [
                "SwiftTreeSitter",
                "TreeSitterQuipu",
            ],
            path: "bindings/swift/TreeSitterQuipuTests"
        )
    ],
    cLanguageStandard: .c11
)
