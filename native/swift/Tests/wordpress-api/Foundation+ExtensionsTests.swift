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

    @Test("A Retry-After header value that's an integer can be parsed")
    func testParsingIntegerRetryAfterHeader() {
        #expect(TimeInterval(3) == TimeInterval.fromRetryHeaderValue("3"))
    }

    @Test("A Retry-After header value that's a date can be parsed")
    func testParsingDateRetryAfterHeader() {

        #expect(TimeInterval(3) == TimeInterval.fromRetryHeaderValue(
            "Thu, 13 Feb 2025 19:57:34 GMT",
            now: Date(timeIntervalSince1970: 1739476651))
        )
    }
}
