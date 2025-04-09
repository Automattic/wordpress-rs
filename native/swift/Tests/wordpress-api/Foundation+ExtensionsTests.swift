import Foundation
import Testing
import WordPressAPI

class FoundationExtensionsTests {

    @Test("The Foundation Extension can parse WordPress-formatted date strings", arguments: [
        "2024-07-04T01:49:37"
    ])
    func testWordPressDateTimeParsing(_ string: String) throws {
        #expect(Date.fromWordPressDate(string) != nil)
    }
}
