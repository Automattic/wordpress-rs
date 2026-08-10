import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

struct WordPressAPITests {

    func createStubs() throws -> HTTPStubs {
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
        return HTTPStubs(stubs: [
            HTTPStubs.stub(path: "/wp-json/wp/v2/users/1", with: try .json(response))
        ])
    }

    @Test
    func testExample() async throws {
        let stubs = try createStubs()
        let api = try WordPressAPI(
            siteInfo: .selfHosted(
                siteUrl: ParsedUrl.parse(input: "https://wordpress.org"),
                apiRoot: ParsedUrl.parse(input: "https://wordpress.org/wp-json")
            ),
            authenticationProvider: .none(),
            executor: stubs,
            middlewarePipeline: .default,
            appNotifier: nil
        )
        let user = try await api.users.retrieveWithViewContext(userId: 1)
        #expect(user.data.name == "User Name")
    }

    @Test
    func testPipeline() async throws {
        let stubs = try createStubs()
        let counter = CounterMiddleware()
        let api = try WordPressAPI(
            siteInfo: .selfHosted(
                siteUrl: ParsedUrl.parse(input: "https://wordpress.org"),
                apiRoot: ParsedUrl.parse(input: "https://wordpress.org/wp-json")
            ),
            authenticationProvider: .none(),
            executor: stubs,
            middlewarePipeline: .init(middlewares: [counter]),
            appNotifier: nil
        )
        _ = try await api.users.retrieveWithViewContext(userId: 1)
        await #expect(counter.count == 1)
    }

    @Test
    func testRoot() async throws {
        let api = try WordPressAPI(
            siteInfo: .selfHosted(
                siteUrl: ParsedUrl.parse(input: "https://vanilla.wpmt.co"),
                apiRoot: ParsedUrl.parse(input: "https://vanilla.wpmt.co/wp-json")
            ),
            authenticationProvider: .none(),
            executor: WpRequestExecutor(urlSession: .shared),
            middlewarePipeline: .default,
            appNotifier: nil
        )

        let details = try await api.apiRoot.get()
        #expect(details.data.siteUrlString() == "https://vanilla.wpmt.co")
    }

    // A refused connection — the host resolves (loopback), but nothing is
    // listening on the port — must be classified as `.connectionError`, matching
    // the Kotlin and reqwest executors. It must *not* be `.nonExistentSiteError`,
    // which is reserved for DNS failures so `isSiteUnreachable` stays a portable
    // "the host does not resolve" signal across platforms. See #1495.
    @Test(.enabled(if: !isLinux()))
    func testRefusedConnectionIsConnectionError() async throws {
        // Port 1 on loopback is privileged, so nothing is bound in any test
        // environment, and the OS refuses the connection immediately
        // (`URLError.cannotConnectToHost`).
        let api = try WordPressAPI(
            siteInfo: .selfHosted(
                siteUrl: ParsedUrl.parse(input: "http://127.0.0.1:1"),
                apiRoot: ParsedUrl.parse(input: "http://127.0.0.1:1/wp-json")
            ),
            authenticationProvider: .none(),
            executor: WpRequestExecutor(urlSession: .init(configuration: .ephemeral)),
            middlewarePipeline: .default,
            appNotifier: nil
        )

        await #expect(
            performing: {
                _ = try await api.apiRoot.get()
            },
            throws: { error in
                guard
                    let apiError = error as? WpApiError,
                    case .RequestExecutionFailed(
                        statusCode: _,
                        redirects: _,
                        reason: let reason,
                        requestUrl: _,
                        requestMethod: _
                    ) = apiError
                else {
                    Issue.record("Expected WpApiError.RequestExecutionFailed, got: \(error)")
                    return false
                }

                guard case .connectionError = reason else {
                    Issue.record("A refused connection must be `.connectionError`, got: \(reason)")
                    return false
                }

                return true
            }
        )
    }
}

private actor CounterMiddleware: Middleware {
    var count = 0

    func process(
        requestExecutor: RequestExecutor,
        response: WpNetworkResponse,
        request: WpNetworkRequest,
        context: RequestContext?
    ) async throws -> WpNetworkResponse {
        count += 1
        return response
    }
}
