import XCTest
import wordpress_api

final class WPRestAPIUrlTests: XCTestCase {

    func testThatValidUrlCanBeParsed() throws {
        XCTAssertEqual(URL(string: "http://example.com"), try WpRestApiurl(stringValue: "http://example.com").asUrl())
    }

    func testThatInvalidUrlThrowsError() throws {
        XCTAssertThrowsError(try WpRestApiurl(stringValue: "invalid").asUrl())
    }
}
