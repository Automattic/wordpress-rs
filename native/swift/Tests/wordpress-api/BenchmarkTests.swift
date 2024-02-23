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

struct WPEditContextPost: Codable {
    let id: UInt64
    let date: String
    let dateGmt: String
    let guid: WPGuid
    let modified: String
    let modifiedGmt: String
    let password: String?
    let slug: String
    let status: String
    let link: String
    let title: WPTitle
    let content: WPContent
    let excerpt: WPExcerpt
    let author: UInt64
    let featuredMedia: UInt64
    let commentStatus: String
    let pingStatus: String
    let sticky: Bool
    let template: String
    let format: String
    let meta: WPMeta
    let categories: [UInt64]
    let tags: [UInt64]

    enum CodingKeys: String, CodingKey {
        case id
        case date
        case dateGmt = "date_gmt"
        case guid
        case modified
        case modifiedGmt = "modified_gmt"
        case password
        case slug
        case status
        case link
        case title
        case content
        case excerpt
        case author

        case featuredMedia = "featured_media"
        case commentStatus = "comment_status"
        case pingStatus    = "ping_status"

        case sticky
        case template
        case format
        case categories
        case tags
        case meta
    }
}

struct WPGuid: Codable {
    let raw: String?
    let rendered: String?
}

struct WPTitle: Codable {
    let raw: String?
    let rendered: String?
}

struct WPContent: Codable {
    let raw: String?
    let rendered: String?
    let protected: Bool?
    let blockVersion: Int?

    enum CodingKeys: String, CodingKey {
        case raw
        case rendered
        case protected
        case blockVersion = "block_version"
    }
}

struct WPExcerpt: Codable {
    let raw: String?
    let rendered: String?
    let protected: Bool?
}

struct WPMeta: Codable {
    let footnoes: String?
}
