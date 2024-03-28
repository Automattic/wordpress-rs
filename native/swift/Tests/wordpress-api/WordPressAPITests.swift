import XCTest
import Foundation
import wordpress_api
import wordpress_api_wrapper

final class WordPressAPITests: XCTestCase {

    func testExample() {
        let request = WpApiHelper(siteUrl: "https://wordpress.org", authentication: .none)
            .postListRequest(params: .init())
        XCTAssertTrue(request.url.hasPrefix("https://wordpress.org/wp-json/wp/v2/posts"))
    }

    func testPaginator() async throws {
        let api = WordPressAPI(urlSession: .shared, baseUrl: URL(string: "https://instant-unknown-banana.jurassic.ninja")!, authenticationStategy: .init(username: "demo", password: "OpYcWbQezJ30vk83ChE4"))
        let pages: [[PostObject]] = try await api.list(type: PostObject.self, perPage: 10).reduce(into: []) { $0.append($1) }
        let total = pages.flatMap { $0 }
        XCTAssertEqual(pages.count, 4)
        XCTAssertEqual(total.count, 36)
    }

    func testNativeError() async {
        let api = WordPressAPI(urlSession: .shared, baseUrl: URL(string: "http://a-url-that-do-not-exists.local")!, authenticationStategy: .none)
        do {
            let _ = try await api.list(type: PostObject.self, perPage: 10).reduce(into: []) { $0.append($1) }
            XCTFail("The above call should throw")
        } catch {
            XCTAssertTrue(error is URLError)
        }
    }
}
