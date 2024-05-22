import XCTest
import Foundation
import WordPressAPI
import WordPressAPIInternal

final class WordPressAPITests: XCTestCase {

    func testExample() {
        let request = WpApiHelper(siteUrl: "https://wordpress.org", authentication: .none)
            .listUsersRequest(context: .view, params: nil)
        XCTAssertTrue(request.url.hasPrefix("https://wordpress.org/wp-json/wp/v2/users"))
    }
}
