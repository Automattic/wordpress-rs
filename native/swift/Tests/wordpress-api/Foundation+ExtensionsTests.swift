import XCTest
import WordPressAPI

final class FoundationExtensionsTests: XCTestCase {
    
    func testWordPressDateTimeParsing() throws {
        XCTAssertNotNil(Date.fromWordPressDate("2024-07-04T01:49:37"))
    }
}
