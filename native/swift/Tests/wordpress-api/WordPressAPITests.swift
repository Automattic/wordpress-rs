import XCTest
import Foundation
@testable import WordPressAPI

#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

final class WordPressAPITests: XCTestCase {

    func testExample() async throws {
        let response = """
          {
            "id": 1,
            "name": "User Name",
            "url": "",
            "description": "",
            "link": "https://profiles.wordpress.org/user/",
            "slug": "poliuk",
            "avatar_urls": {
              "24": "https://secure.gravatar.com/avatar/uuid?s=24&d=mm&r=g",
              "48": "https://secure.gravatar.com/avatar/uuid?s=48&d=mm&r=g",
              "96": "https://secure.gravatar.com/avatar/uuid?s=96&d=mm&r=g"
            },
            "meta": [],
            "_links": {
              "self": [
                {
                  "href": "https://wordpress.org/wp-json/wp/v2/users/1"
                }
              ],
              "collection": [
                {
                  "href": "https://wordpress.org/wp-json/wp/v2/users"
                }
              ]
            }
          }
        """
        let stubs = HTTPStubs()
        try stubs.stub(path: "/wp-json/wp/v2/users/1", with: .json(response))

        let api = try WordPressAPI(
            urlSession: .shared,
            baseUrl: ParsedUrl.parse(input: "https://wordpress.org"),
            authenticationStategy: .none,
            executor: stubs
        )
        let user = try await api.users.retrieveWithViewContext(userId: 1)
        XCTAssertEqual(user.data.name, "User Name")
    }

#if !os(Linux)
    // Skip on Linux, because `XCTExpectFailure` is unavailable on Linux
    func testTimeout() async throws {
        let stubs = HTTPStubs()
        stubs.missingStub = .failure(URLError(.timedOut))

        let api = try WordPressAPI(
            urlSession: .shared,
            baseUrl: ParsedUrl.parse(input: "https://wordpress.org"),
            authenticationStategy: .none,
            executor: stubs
        )

        do {
            _ = try await api.users.retrieveWithViewContext(userId: 1)
            XCTFail("Unexpected response")
        } catch let error as URLError {
            XCTAssertEqual(error.code, .timedOut)
        } catch {
            #if canImport(WordPressAPIInternal)
            XCTAssertTrue(error is WordPressAPIInternal.WpApiError)
            #endif
        }
    }
#endif

}
