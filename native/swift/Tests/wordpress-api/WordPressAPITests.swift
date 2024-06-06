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

#if !os(Linux)
    // Skip on Linux, because `XCTExpectFailure` is unavailable on Linux
    func testTimeout() async throws {
        let stubs = HTTPStubs()
        stubs.missingStub = .failure(URLError(.timedOut))

        let api = try WordPressAPI(
            urlSession: .shared,
            baseUrl: URL(string: "https://wordpress.org")!,
            authenticationStategy: .none,
            executor: stubs
        )

        do {
            _ = try await api.users.retrieveWithViewContext(userId: 1)
            XCTFail("Unexpected response")
        } catch let error as URLError {
            XCTAssertEqual(error.code, .timedOut)
        } catch {
            XCTExpectFailure("URLError can't not be passed to Rust")
            XCTAssertFalse(true, "Unexpected error: \(error)")
        }
    }
#endif

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

class HTTPStubs: SafeRequestExecutor {

    var stubs: [(condition: (WpNetworkRequest) -> Bool, response: WpNetworkResponse)] = []

    var missingStub: Result<WpNetworkResponse, Error>?

    public func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError> {
        if let response = stub(for: request) {
            return .success(response)
        }

        switch missingStub {
        case let .success(response):
            return .success(response)
        case .failure:
            // TODO: Translate error into the Rust type
            return .failure(.RequestExecutionFailed(statusCode: nil, reason: ""))
        default:
            // TODO: Translate error into the Rust type
            return .failure(.RequestExecutionFailed(statusCode: nil, reason: ""))
        }
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
