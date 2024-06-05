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
        stubs.stub(path: "/wp-json/wp/v2/users/1", with: .json(response))

        let api = try WordPressAPI(
            urlSession: .shared,
            baseUrl: URL(string: "https://wordpress.org")!,
            authenticationStategy: .none,
            executor: stubs
        )
        let user = try await api.users.retrieveWithViewContext(userId: 1)
        XCTAssertEqual(user.name, "User Name")
    }

}

extension WpNetworkResponse {
    static func json(_ content: String) -> WpNetworkResponse {
        WpNetworkResponse(
            body: content.data(using: .utf8)!,
            statusCode: 200,
            headerMap: ["Content-Type": "application/json"]
        )
    }
}

class HTTPStubs: RequestExecutor {

    var stubs: [(condition: (WpNetworkRequest) -> Bool, response: WpNetworkResponse)] = []

    func execute(request: WpNetworkRequest) async throws -> WpNetworkResponse {
        stub(for: request) ?? WpNetworkResponse(body: Data(), statusCode: 404, headerMap: nil)
    }

    func stub(for request: WpNetworkRequest) -> WpNetworkResponse? {
        stubs.first { stub in stub.condition(request) }?
            .response
    }

    func stub(path: String, with response: WpNetworkResponse) {
        stubs.append((
            condition: { URL(string: $0.url)?.path == path },
            response: response
        ))
    }

}
