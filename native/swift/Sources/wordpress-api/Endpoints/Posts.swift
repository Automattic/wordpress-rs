import Foundation
import wordpress_api_wrapper

extension SparsePost: Contextual {
    public typealias ID = PostId
    public typealias ViewContext = PostWithViewContext
    public typealias EditContext = PostWithEditContext
    public typealias EmbedContext = PostWithEmbedContext

    public static func retrieveRequest(id: PostId, using helper: WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest {
        helper.retrievePostRequest(postId: id, context: context)
    }

    public static func listRequest(params: PostListParams?, using helper: WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest {
        helper.postListRequest(params: params ?? .init())
    }

    public static func updateRequest(id: PostId, params: PostUpdateParams, using helper: any WpApiHelperProtocol) -> WpNetworkRequest {
        helper.updatePostRequest(postId: id, params: params)
    }

    public static func createRequest(params: PostCreateParams, using helper: any WpApiHelperProtocol) -> WpNetworkRequest {
        helper.createPostRequest(params: params)
    }

    public static func deleteRequest(id: ID, params: PostDeleteParams, using helper: WpApiHelperProtocol) -> WpNetworkRequest {
        helper.deletePostRequest(postId: id, params: params)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> PostWithViewContext {
        try parseRetrievePostResponseWithViewContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> PostWithEditContext {
        try parseRetrievePostResponseWithEditContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> PostWithEmbedContext {
        try parseRetrievePostResponseWithEmbedContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> [PostWithViewContext] {
        try parseListPostsResponseWithViewContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> [PostWithEditContext] {
        try parseListPostsResponseWithEditContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> [PostWithEmbedContext] {
        try parseListPostsResponseWithEmbedContext(response: response)
    }
}

extension WordPressAPI {
    public var posts: AnyNamespace<SparsePost> {
        .init(api: self)
    }
}
