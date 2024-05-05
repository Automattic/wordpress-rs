import XCTest
import Foundation
import WordPressAPI
import WordPressAPIInternal

final class WordPressAPITests: XCTestCase {

    func testExample() {
        let request = WpApiHelper(siteUrl: "https://wordpress.org", authentication: .none)
            .postListRequest(params: .init())
        XCTAssertTrue(request.url.hasPrefix("https://wordpress.org/wp-json/wp/v2/posts"))
    }
}
