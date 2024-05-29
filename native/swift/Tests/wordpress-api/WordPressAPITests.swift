import XCTest
import Foundation
import WordPressAPI
#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

final class WordPressAPITests: XCTestCase {

    func testExample() throws {
        let request = try WpRequestBuilder(siteUrl: "https://wordpress.org", authentication: .none)
            .users().list(context: .view, params: nil) 
        XCTAssertTrue(XCTUnwrap(request).url.hasPrefix("https://wordpress.org/wp-json/wp/v2/users"))
    }
}
