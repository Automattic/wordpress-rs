import Foundation
import wordpress_api_wrapper

extension WordPressAPI {

    // MARK: Structured Concurrency

    /// Fetch a list of posts
    ///
    /// If you're only interested in fetching a specific page, this is a good method for that – if you
    /// want to sync all records,  consider using the overload of this method that returns `SparsePostSequence`.
    public func listPosts(params: PostListParams = PostListParams()) async throws -> PostListResponse {
        let request = self.helper.postListRequest(params: params)
        let response = try await perform(request: request)
        return try parsePostListResponse(response: response)
    }

    /// A good way to fetch every post (you can still specify a specific offset using `params`)
    ///
    public func listPosts(params: PostListParams = PostListParams()) -> SparsePostSequence {
        SparsePostSequence(api: self, initialParams: params)
    }

    package func listPosts(url: String) async throws -> PostListResponse {
        let request = self.helper.rawRequest(url: url)
        let response = try await perform(request: request)
        return try parsePostListResponse(response: response)
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
