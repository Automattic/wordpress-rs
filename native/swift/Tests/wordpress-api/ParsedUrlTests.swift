import Foundation
import XCTest
import WordPressAPI

class ParsedUrlTests: XCTestCase {

    func testRoundTrip() throws {
        let urls = [
            "http://example.com",
            "https://www.example.com/path/to/resource",
            "https://example.com/search?q=unit+testing&sort=asc",
            "https://example.com/index.html#section",
            "http://example.com:8080/path",
            "https://subdomain.example.com",
            "http://user:password@example.com",
            "file:///home/user/file.txt",
            "ftp://ftp.example.com/resource.txt",
            "http://[2001:db8::1]:8080/"
        ]

        for url in urls {
            let parsedUrl = try ParsedUrl.parse(input: url)
            XCTAssertEqual(parsedUrl.asURL().absoluteString, url)
        }
    }

}
