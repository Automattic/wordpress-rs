import Foundation
import Combine
@preconcurrency import WordPressAPIInternal

public final class PostsRequestPerformer {
    typealias ExecutorType = PostsRequestExecutor
    typealias RequestBuilderType = PostsRequestBuilder

    let executor: PostsRequestExecutor
    let builder: RequestBuilderType
    public let urlSession: URLSession

    init(executor: PostsRequestExecutor, builder: RequestBuilderType, session: URLSession) {
        self.executor = executor
        self.builder = builder
        self.urlSession = session
    }
}

extension PostsRequestPerformer: PublisherAwarePerformer {}
extension PostsRequestPerformer: CallbackAwarePerformer {}

extension PostsRequestPerformer: RequestPerformer {
    public typealias IdType = PostId

    public typealias SingleEditType = PostWithEditContext
    public typealias SingleEmbedType = PostWithEmbedContext
    public typealias SingleViewType = PostWithViewContext

    public typealias ListParamsType = PostListParams
    public typealias CreateParamsType = PostCreateParams
    public typealias UpdateParamsType = PostUpdateParams

    public typealias CreateResponseType = PostsRequestCreateResponse
    public typealias UpdateResponseType = PostsRequestUpdateResponse
    public typealias DeleteResponseType = PostsRequestDeleteResponse

    public typealias EditContextListResponseType = PostsRequestListWithEditContextResponse
    public typealias EmbedContextListResponseType = PostsRequestListWithEmbedContextResponse
    public typealias ViewContextListResponseType = PostsRequestListWithViewContextResponse

    public func buildCreateRequest(params: PostCreateParams) -> WpNetworkRequest {
        builder.create(params: params)
    }

    public func buildUpdateRequest(id: PostId, params: PostUpdateParams) -> WpNetworkRequest {
        builder.update(postId: id, params: params)
    }

    public func buildDeleteRequest(id: PostId) -> WpNetworkRequest {
        builder.delete(postId: id)
    }

    public func buildListWithEditRequest(params: PostListParams) -> WpNetworkRequest {
        builder.listWithEditContext(params: params)
    }

    public func buildListWithEmbedRequest(params: PostListParams) -> WpNetworkRequest {
        builder.listWithEmbedContext(params: params)
    }

    public func buildListWithViewRequest(params: PostListParams) -> WpNetworkRequest {
        builder.listWithViewContext(params: params)
    }

    public func parseCreateResponse(response: WpNetworkResponse) throws -> CreateResponseType {
        try parseAsPostsRequestCreateResponse(response: response)
    }

    public func parseUpdateResponse(response: WpNetworkResponse) throws -> PostsRequestUpdateResponse {
        try parseAsPostsRequestUpdateResponse(response: response)
    }

    public func parseDeleteResponse(response: WpNetworkResponse) throws -> PostsRequestDeleteResponse {
        try parseAsPostsRequestDeleteResponse(response: response)
    }

    public func parseListWithEditResponse(
        response: WpNetworkResponse
    ) throws -> PostsRequestListWithEditContextResponse {
        try parseAsPostsRequestListWithEditContextResponse(response: response)
    }

    public func parseListWithEmbedResponse(
        response: WpNetworkResponse
    ) throws -> PostsRequestListWithEmbedContextResponse {
        try parseAsPostsRequestListWithEmbedContextResponse(response: response)
    }

    public func parseListWithViewResponse(
        response: WpNetworkResponse
    ) throws -> PostsRequestListWithViewContextResponse {
        try parseAsPostsRequestListWithViewContextResponse(response: response)
    }
}

// MARK: - PostsRequestExecutorProtocol

// Allows the performer to respond to everything the PostsRequestExecutor does
extension PostsRequestPerformer: PostsRequestExecutorProtocol {

    public func create(
        params: WordPressAPIInternal.PostCreateParams
    ) async throws -> WordPressAPIInternal.PostsRequestCreateResponse {
        try await self.executor.create(params: params)
    }

    public func update(
        postId: PostId,
        params: PostUpdateParams
    ) async throws -> PostsRequestUpdateResponse {
        try await self.executor.update(postId: postId, params: params)
    }

    public func delete(
        postId: PostId
    ) async throws -> PostsRequestDeleteResponse {
        try await self.executor.delete(postId: postId)
    }

    public func listWithEditContext(params: PostListParams) async throws -> PostsRequestListWithEditContextResponse {
        try await executor.listWithEditContext(params: params)
    }

    public func listWithEmbedContext(params: PostListParams) async throws -> PostsRequestListWithEmbedContextResponse {
        try await executor.listWithEmbedContext(params: params)
    }

    public func listWithViewContext(params: PostListParams) async throws -> PostsRequestListWithViewContextResponse {
        try await executor.listWithViewContext(params: params)
    }

    public func retrieveWithEditContext(
        postId: PostId, params: PostRetrieveParams
    ) async throws -> PostsRequestRetrieveWithEditContextResponse {
        try await executor.retrieveWithEditContext(postId: postId, params: params)
    }

    public func retrieveWithEmbedContext(
        postId: PostId, params: PostRetrieveParams
    ) async throws -> PostsRequestRetrieveWithEmbedContextResponse {
        try await executor.retrieveWithEmbedContext(postId: postId, params: params)
    }

    public func retrieveWithViewContext(
        postId: PostId, params: PostRetrieveParams
    ) async throws -> PostsRequestRetrieveWithViewContextResponse {
        try await executor.retrieveWithViewContext(postId: postId, params: params)
    }

    public func filterListWithEditContext(
        params: PostListParams, fields: [SparsePostFieldWithEditContext]
    ) async throws -> PostsRequestFilterListWithEditContextResponse {
        try await executor.filterListWithEditContext(params: params, fields: fields)
    }

    public func filterListWithEmbedContext(
        params: PostListParams, fields: [SparsePostFieldWithEmbedContext]
    ) async throws -> PostsRequestFilterListWithEmbedContextResponse {
        try await executor.filterListWithEmbedContext(params: params, fields: fields)
    }

    public func filterListWithViewContext(
        params: PostListParams, fields: [SparsePostFieldWithViewContext]
    ) async throws -> PostsRequestFilterListWithViewContextResponse {
        try await executor.filterListWithViewContext(params: params, fields: fields)
    }

    public func filterRetrieveWithEditContext(
        postId: PostId, params: PostRetrieveParams, fields: [SparsePostFieldWithEditContext]
    ) async throws -> PostsRequestFilterRetrieveWithEditContextResponse {
        try await executor.filterRetrieveWithEditContext(postId: postId, params: params, fields: fields)
    }

    public func filterRetrieveWithEmbedContext(
        postId: PostId, params: PostRetrieveParams, fields: [SparsePostFieldWithEmbedContext]
    ) async throws -> PostsRequestFilterRetrieveWithEmbedContextResponse {
        try await executor.filterRetrieveWithEmbedContext(postId: postId, params: params, fields: fields)
    }

    public func filterRetrieveWithViewContext(
        postId: PostId, params: PostRetrieveParams, fields: [SparsePostFieldWithViewContext]
    ) async throws -> PostsRequestFilterRetrieveWithViewContextResponse {
        try await executor.filterRetrieveWithViewContext(postId: postId, params: params, fields: fields)
    }

    public func trash(postId: PostId) async throws -> PostsRequestTrashResponse {
        try await executor.trash(postId: postId)
    }
}
