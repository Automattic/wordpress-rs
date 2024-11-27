import Foundation

#if canImport(WordPressAPIInternal)
@preconcurrency import WordPressAPIInternal
#endif

#if os(Linux)
import FoundationNetworking
#endif

public final class MediaRequestPerformer {
    typealias ExecutorType = MediaRequestExecutor
    typealias RequestBuilderType = MediaRequestBuilder

    let executor: ExecutorType
    let builder: RequestBuilderType
    public let urlSession: URLSession

    init(executor: ExecutorType, builder: RequestBuilderType, session: URLSession) {
        self.executor = executor
        self.builder = builder
        self.urlSession = session
    }}

public struct MediaCreateParams {}
public struct MediaRequestCreateResponse {}

extension MediaRequestPerformer: CallbackAwarePerformer {}

#if canImport(Combine)
extension MediaRequestPerformer: PublisherAwarePerformer {}
#endif

extension MediaRequestPerformer: RequestPerformer {
    public func buildListWithEditRequest(params: MediaListParams) -> WpNetworkRequest {
        builder.listWithEditContext(params: params)
    }

    public func buildListWithEmbedRequest(params: MediaListParams) -> WpNetworkRequest {
        builder.listWithEmbedContext(params: params)
    }

    public func buildListWithViewRequest(params: MediaListParams) -> WpNetworkRequest {
        builder.listWithViewContext(params: params)
    }

    public func parseCreateResponse(response: WpNetworkResponse) throws -> MediaRequestCreateResponse {
        MediaRequestCreateResponse() // TODO
    }

    public func parseUpdateResponse(response: WpNetworkResponse) throws -> MediaRequestUpdateResponse {
        try parseAsMediaRequestUpdateResponse(response: response)
    }

    public func parseDeleteResponse(response: WpNetworkResponse) throws -> MediaRequestDeleteResponse {
        try parseAsMediaRequestDeleteResponse(response: response)
    }

    public typealias IdType = MediaId

    public typealias SingleEditType = MediaWithEditContext
    public typealias SingleEmbedType = MediaWithEmbedContext
    public typealias SingleViewType = MediaWithViewContext

    public typealias ListParamsType = MediaListParams
    public typealias CreateParamsType = MediaCreateParams
    public typealias UpdateParamsType = MediaUpdateParams

    public typealias CreateResponseType = MediaRequestCreateResponse
    public typealias UpdateResponseType = MediaRequestUpdateResponse
    public typealias DeleteResponseType = MediaRequestDeleteResponse

    public typealias EditContextListResponseType = MediaRequestListWithEditContextResponse
    public typealias EmbedContextListResponseType = MediaRequestListWithEmbedContextResponse
    public typealias ViewContextListResponseType = MediaRequestListWithViewContextResponse

    public func buildCreateRequest(params: MediaCreateParams) -> WpNetworkRequest {
        WpNetworkRequest(noPointer: .init())
    }

    public func buildUpdateRequest(id: MediaId, params: MediaUpdateParams) -> WpNetworkRequest {
        builder.update(mediaId: id, params: params)
    }

    public func buildDeleteRequest(id: MediaId) -> WpNetworkRequest {
        builder.delete(mediaId: id)
    }

    public func parseListWithEditResponse(
        response: WpNetworkResponse
    ) throws -> MediaRequestListWithEditContextResponse {
        try parseAsMediaRequestListWithEditContextResponse(response: response)
    }

    public func parseListWithEmbedResponse(
        response: WpNetworkResponse
    ) throws -> MediaRequestListWithEmbedContextResponse {
        try parseAsMediaRequestListWithEmbedContextResponse(response: response)
    }

    public func parseListWithViewResponse(
        response: WpNetworkResponse
    ) throws -> MediaRequestListWithViewContextResponse {
        try parseAsMediaRequestListWithViewContextResponse(response: response)
    }
}

// MARK: - PostsRequestExecutorProtocol

// Allows the performer to respond to everything the PostsRequestExecutor does
extension MediaRequestPerformer: MediaRequestExecutorProtocol {
    public func listWithViewContext(
        params: MediaListParams
    ) async throws -> MediaRequestListWithViewContextResponse {
        try await self.executor.listWithViewContext(params: params)
    }

    public func listWithEmbedContext(
        params: MediaListParams
    ) async throws -> MediaRequestListWithEmbedContextResponse {
        try await self.executor.listWithEmbedContext(params: params)
    }

    public func listWithEditContext(
        params: MediaListParams
    ) async throws -> MediaRequestListWithEditContextResponse {
        try await self.executor.listWithEditContext(params: params)
    }

    public func delete(
        mediaId: MediaId
    ) async throws -> MediaRequestDeleteResponse {
        try await self.executor.delete(mediaId: mediaId)
    }

    public func filterListWithEditContext(
        params: MediaListParams,
        fields: [SparseMediaFieldWithEditContext]
    ) async throws -> MediaRequestFilterListWithEditContextResponse {
        try await self.executor.filterListWithEditContext(params: params, fields: fields)
    }

    public func filterListWithEmbedContext(
        params: MediaListParams,
        fields: [SparseMediaFieldWithEmbedContext]
    ) async throws -> MediaRequestFilterListWithEmbedContextResponse {
        try await self.executor.filterListWithEmbedContext(params: params, fields: fields)
    }

    public func filterListWithViewContext(
        params: MediaListParams,
        fields: [SparseMediaFieldWithViewContext]
    ) async throws -> MediaRequestFilterListWithViewContextResponse {
        try await self.executor.filterListWithViewContext(params: params, fields: fields)
    }

    public func filterRetrieveWithEditContext(
        mediaId: MediaId,
        fields: [SparseMediaFieldWithEditContext]
    ) async throws -> MediaRequestFilterRetrieveWithEditContextResponse {
        try await self.executor.filterRetrieveWithEditContext(mediaId: mediaId, fields: fields)
    }

    public func filterRetrieveWithEmbedContext(
        mediaId: MediaId,
        fields: [SparseMediaFieldWithEmbedContext]
    ) async throws -> MediaRequestFilterRetrieveWithEmbedContextResponse {
        try await self.executor.filterRetrieveWithEmbedContext(mediaId: mediaId, fields: fields)
    }

    public func filterRetrieveWithViewContext(
        mediaId: MediaId,
        fields: [SparseMediaFieldWithViewContext]
    ) async throws -> MediaRequestFilterRetrieveWithViewContextResponse {
        try await self.executor.filterRetrieveWithViewContext(mediaId: mediaId, fields: fields)
    }

    public func retrieveWithEditContext(
        mediaId: MediaId
    ) async throws -> MediaRequestRetrieveWithEditContextResponse {
        try await self.executor.retrieveWithEditContext(mediaId: mediaId)
    }

    public func retrieveWithEmbedContext(
        mediaId: MediaId
    ) async throws -> MediaRequestRetrieveWithEmbedContextResponse {
        try await self.executor.retrieveWithEmbedContext(mediaId: mediaId)
    }

    public func retrieveWithViewContext(
        mediaId: MediaId
    ) async throws -> MediaRequestRetrieveWithViewContextResponse {
        try await self.executor.retrieveWithViewContext(mediaId: mediaId)
    }

    public func update(
        mediaId: MediaId,
        params: MediaUpdateParams
    ) async throws -> MediaRequestUpdateResponse {
        try await self.executor.update(mediaId: mediaId, params: params)
    }
}

extension MediaRequestListWithEditContextResponse: PaginatableResponse, @unchecked Sendable {}
extension MediaRequestListWithViewContextResponse: PaginatableResponse, @unchecked Sendable {}
extension MediaRequestListWithEmbedContextResponse: PaginatableResponse, @unchecked Sendable {}
