import Foundation
import wordpress_api_wrapper

extension WordPressAPI {

    // MARK: Structured Concurrency

    /// Fetch a list of posts
    ///
    /// If you're only interested in fetching a specific page, this is a good method for that – if you want to sync all records, consider using the
    /// overload of this method that returns `PostObjectSequence`.
    public func listPosts(params: PostListParams = PostListParams()) async throws -> PostListResponse {
        let request = self.helper.postListRequest(params: params)
        let response = try await perform(request: request)
        return try parsePostListResponse(response: response)
    }


    /// A good way to fetch every post (you can still specify a specific offset using `params`)
    ///
    public func listPosts(params: PostListParams = PostListParams(page: 1, perPage: 99)) -> PostObjectSequence {
        PostObjectSequence(api: self, initialParams: params)
    }

    public func listPostsRust(
        params: PostListParams = PostListParams(page: 1, perPage: 99)
    ) async throws -> [PostObject] {
        let request = self.helper.postListRequest(params: params)
        let response = try await perform(request: request)

        let startTime = CFAbsoluteTimeGetCurrent()
        let parsedResponse = try parsePostListResponse(response: response)
        let timeElapsed = CFAbsoluteTimeGetCurrent() - startTime
        debugPrint("Rust Implementation: \(timeElapsed)")

        return parsedResponse.postList ?? []
    }

    public func listPostsNative(
        params: PostListParams = PostListParams(page: 1, perPage: 99)
    ) async throws -> [WPEditContextPost] {
        let request = self.helper.postListRequest(params: params)
        let response = try await perform(request: request)

        let startTime = CFAbsoluteTimeGetCurrent()
        let posts = try JSONDecoder().decode([WPEditContextPost].self, from: response.body)
        let timeElapsed = CFAbsoluteTimeGetCurrent() - startTime
        debugPrint("Swift Implementation: \(timeElapsed)")

        return posts
    }

    package func listPosts(url: String) async throws -> PostListResponse {
        let request = self.helper.rawRequest(url: url)
        let response = try await perform(request: request)

        let startTime = CFAbsoluteTimeGetCurrent()
        let parsedResponse = try parsePostListResponse(response: response)
        let timeElapsed = CFAbsoluteTimeGetCurrent() - startTime
        debugPrint(timeElapsed)

        return parsedResponse
    }

    // MARK: Callbacks
    public typealias ListPostsCallback = (Result<PostListResponse, Error>) -> Void

    public func listPosts(params: PostListParams = PostListParams(), callback: @escaping ListPostsCallback) {
        let request = self.helper.postListRequest(params: params)

        self.perform(request: request) { result in
            let parseResult = result.tryMap { try parsePostListResponse(response: $0) }
            callback(parseResult)
        }
    }
}


public struct WPEditContextPost: Codable {
    public let id: UInt64
    let date: String
    let dateGmt: String
    let guid: WPGuid
    let modified: String
    let modifiedGmt: String
    let password: String?
    let slug: String
    let status: String
    let link: String
    public let title: WPTitle
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

public struct WPGuid: Codable {
    let raw: String?
    let rendered: String?
}

public struct WPTitle: Codable {
    public let raw: String?
    public let rendered: String?
}

public struct WPContent: Codable {
    public let raw: String?
    public let rendered: String?
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
