import Foundation
import XCTest

@testable import WordPressAPI

#if canImport(WordPressAPIInternal)
@testable import WordPressAPIInternal
#endif

class URLTests: XCTestCase {

    func testParseApiBaseUrl() throws {
        let urls = try [
                "http://example.com/path?query=value#fragment",
                "http://example.com:8080/path",
                "http://sub.sub2.example.com/path",
                "http://192.168.1.1/path",
                "http://example.com/a/very/long/path/that/goes/on/forever",
                "http://example.com/path%20with%20spaces",
                "http://example.com/~user!$&'()*+,;=:@/path",
                "http://user:password@example.com/path",
                "http://example.com",
                "http://example.com./path"
            ]
            .map { try XCTUnwrap(URL(string: $0)) }
        for url in urls {
            XCTAssertTrue(apiBaseUrlFromStr(str: url.absoluteString) != nil, "Invalid URL: \(url)")
        }
    }

}
