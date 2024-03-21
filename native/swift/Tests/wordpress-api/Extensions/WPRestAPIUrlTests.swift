import XCTest
import wordpress_api
import wordpress_api_wrapper // We need to construct internal types to test them properly

final class WPRestAPIUrlTests: XCTestCase {

    func testThatValidUrlCanBeParsed() throws {
        XCTAssertEqual(URL(string: "http://example.com"), try WpRestApiurl(stringValue: "http://example.com").asUrl())
    }

    func testThatInvalidUrlThrowsError() throws {
        XCTAssertThrowsError(try WpRestApiurl(stringValue: "invalid").asUrl())
    }
}
