import XCTest
import Foundation
import WordPressAPI
import WordPressAPIInternal

final class WordPressAPITests: XCTestCase {

    func testExample() {
        let request = WpRequestBuilder(siteUrl: "https://wordpress.org", authentication: .none)
            .users().list(context: .view, params: nil)
        XCTAssertTrue(request.url.hasPrefix("https://wordpress.org/wp-json/wp/v2/users"))
    }
}
