import Foundation
import wordpress_api_wrapper

extension SparsePost: Contextual {
    public typealias ID = PostId
    public typealias View = PostWithViewContext
    public typealias Edit = PostWithEditContext
    public typealias Embed = PostWithEmbedContext

    public static func makeGetOneRequest(id: PostId, using helper: WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest {
        helper.retrievePostRequest(postId: id, context: context)
    }

    public static func makeGetListRequest(using helper: WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest {
        helper.postListRequest(params: .init())
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
