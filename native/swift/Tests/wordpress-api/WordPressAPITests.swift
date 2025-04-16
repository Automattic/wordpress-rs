import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

struct WordPressAPITests {

    @Test
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
        let stubs = HTTPStubs(stubs: [
            HTTPStubs.stub(path: "/wp-json/wp/v2/users/1", with: try .json(response))
        ])

        let api = try WordPressAPI(
            apiRootUrl: ParsedUrl.parse(input: "https://wordpress.org/wp-json"),
            authenticationStategy: .none,
            executor: stubs
        )
        let user = try await api.users.retrieveWithViewContext(userId: 1)
        #expect(user.data.name == "User Name")
    }

    @Test
    func testPipeline() async throws {
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
        let stubs = HTTPStubs(stubs: [
            HTTPStubs.stub(path: "/wp-json/wp/v2/users/1", with: try .json(response))
        ])

        let counter = CounterMiddleware()
        let api = try WordPressAPI(
            apiRootUrl: ParsedUrl.parse(input: "https://wordpress.org/wp-json"),
            authenticationStategy: .none,
            executor: stubs,
            middlewarePipeline: .init(middlewares: [counter])
        )
        let _ = try await api.users.retrieveWithViewContext(userId: 1)
        await #expect(counter.count == 1)
    }
}

private actor CounterMiddleware: Middleware {
    var count = 0

    func process(requestExecutor: RequestExecutor, response: WpNetworkResponse, request: WpNetworkRequest) async throws  -> WpNetworkResponse {
        count += 1
        return response
    }
}
