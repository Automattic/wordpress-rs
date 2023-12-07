import Foundation

import wordpress_api_wrapper

public struct WordPressAPI {

    private let parser: PostResponseParser

    public init(parser: PostResponseParser = postResponseParser()) {
        self.parser = parser
    }

    public func listPosts(params: PostListParams = .default) async throws -> ParsedPostListResponse {
        let request = PostRequestBuilder().list(params: params) // TODO: Get the request stuff over into a `URLRequest`

        let _request = URLRequest(url: URL(string: "https://public-api.wordpress.com")!)
        let (data, response) = try await URLSession.shared.data(for: _request)

        let postListResponse = PostListResponse() // TODO: Figure out how to get the data from an HTTP response into this object
        return parser.list(response: postListResponse)
    }

    public func combineStrings(_ lhs: String, _ rhs: String) -> String {
        wordpress_api_wrapper.combineStrings(a: lhs, b: rhs)
    }
}

public extension PostListParams {
    static let `default` = PostListParams(page: 1, perPage: 10)
}