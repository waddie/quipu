import XCTest
import SwiftTreeSitter
import TreeSitterQuipu

final class TreeSitterQuipuTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_quipu())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading quipu grammar")
    }
}
