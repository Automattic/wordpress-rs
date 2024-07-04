import Foundation
import XCTest

@testable import WordPressAPI

final class UsersTest: XCTestCase {

    func testRetrieveUser() async throws {
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
            baseUrl: WpParsedUrl.parse(input: "https://wordpress.org"),
            authenticationStategy: .none,
            executor: stubs
        )
        let user = try await api.users.retrieveWithViewContext(userId: 1)
        XCTAssertEqual(user.name, "User Name")
    }

}
