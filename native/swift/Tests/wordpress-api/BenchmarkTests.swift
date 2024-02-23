import Foundation
import XCTest
@testable import wordpress_api
import wordpress_api_wrapper

final class BenchmarkTests: XCTestCase {

    private let iterationCount = 100
    private let postListParams = PostListParams(page: 1, perPage: 10)

    private let siteUrl = URL(string: "{redacted}")!
    private let authentication = WpAuthentication(authToken: "{redacted}")
    private var wordpressApi: WordPressAPI!

    override func setUp() {
        self.wordpressApi = WordPressAPI(urlSession: .shared, baseUrl: siteUrl, authenticationStategy: authentication)
    }

    func testRustRequestCreationSpeed() async throws {
        measure {
            for _ in 1...iterationCount {
                _ = wordpressApi.helper.postListRequest(params: postListParams).asURLRequest()
            }
        }
    }

    @available(iOS 16.0, *)
    func testSwiftRequestCreationSpeed() async throws {
        measure {
            for _ in 1...iterationCount {
                var request = URLRequest(url: siteUrl.appendingPathComponent("/wp-rest/v1/posts").appending(queryItems: [
                    URLQueryItem(name: "page", value: "\(postListParams.page ?? 1)"),
                    URLQueryItem(name: "per_page", value: "\(postListParams.perPage ?? 10)")
                ]))
                request.setValue(authentication.authToken, forHTTPHeaderField: "Authorization")
            }
        }
    }

    func testRustParsingSpeed() async throws {
        let url = Bundle.module.url(forResource: "raw", withExtension: "json")!
        let data = try Data(contentsOf: url)
        let response = WpNetworkResponse(body: data, statusCode: 200, headerMap: nil)

        measure {
            for _ in 1...iterationCount {
                try! parsePostListResponse(response: response)
            }
        }
    }

    func testSwiftParsingSpeed() async throws {
        let url = Bundle.module.url(forResource: "raw", withExtension: "json")!
        let data = try Data(contentsOf: url)

        measure {
            for _ in 1...iterationCount {
                try! JSONDecoder().decode([WPEditContextPost].self, from: data)
            }
        }
    }
}
