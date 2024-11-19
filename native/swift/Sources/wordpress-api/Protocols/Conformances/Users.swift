import Foundation
import Combine
@preconcurrency import WordPressAPIInternal

public final class UsersRequestPerformer {
    typealias ExecutorType = UsersRequestExecutor
    typealias RequestBuilderType = UsersRequestBuilder

    let executor: UsersRequestExecutor
    let builder: RequestBuilderType
    public let urlSession: URLSession

    init(executor: UsersRequestExecutor, builder: RequestBuilderType, session: URLSession) {
        self.executor = executor
        self.builder = builder
        self.urlSession = session
    }
}

extension UsersRequestPerformer: RequestPerformer, HasDeletionParams {
    public typealias IdType = UserId

    public typealias SingleEditType = UserWithEditContext
    public typealias SingleEmbedType = UserWithEmbedContext
    public typealias SingleViewType = UserWithViewContext

    public typealias ListParamsType = UserListParams
    public typealias CreateParamsType = UserCreateParams
    public typealias UpdateParamsType = UserUpdateParams
    public typealias DeleteParamsType = UserDeleteParams

    public typealias CreateResponseType = UsersRequestCreateResponse
    public typealias UpdateResponseType = UsersRequestUpdateResponse
    public typealias DeleteResponseType = UsersRequestDeleteResponse

    public typealias EditContextListResponseType = UsersRequestListWithEditContextResponse
    public typealias EmbedContextListResponseType = UsersRequestListWithEmbedContextResponse
    public typealias ViewContextListResponseType = UsersRequestListWithViewContextResponse

    public func buildCreateRequest(params: UserCreateParams) -> WpNetworkRequest {
        builder.create(params: params)
    }

    public func buildUpdateRequest(id: UserId, params: UserUpdateParams) -> WpNetworkRequest {
        builder.update(userId: id, params: params)
    }

    public func buildDeleteRequest(id: UserId, params: UserDeleteParams) -> WpNetworkRequest {
        builder.delete(userId: id, params: params)
    }

    public func buildListWithEditRequest(params: UserListParams) -> WpNetworkRequest {
        builder.listWithEditContext(params: params)
    }

    public func buildListWithEmbedRequest(params: UserListParams) -> WpNetworkRequest {
        builder.listWithEmbedContext(params: params)
    }

    public func buildListWithViewRequest(params: UserListParams) -> WpNetworkRequest {
        builder.listWithViewContext(params: params)
    }

    public func parseCreateResponse(response: WpNetworkResponse) throws -> CreateResponseType {
        try parseAsUsersRequestCreateResponse(response: response)
    }

    public func parseUpdateResponse(response: WpNetworkResponse) throws -> UsersRequestUpdateResponse {
        try parseAsUsersRequestUpdateResponse(response: response)
    }

    public func parseDeleteResponse(response: WpNetworkResponse) throws -> UsersRequestDeleteResponse {
        try parseAsUsersRequestDeleteResponse(response: response)
    }

    public func parseListWithEditResponse(
        response: WpNetworkResponse
    ) throws -> UsersRequestListWithEditContextResponse {
        try parseAsUsersRequestListWithEditContextResponse(response: response)
    }

    public func parseListWithEmbedResponse(
        response: WpNetworkResponse
    ) throws -> UsersRequestListWithEmbedContextResponse {
        try parseAsUsersRequestListWithEmbedContextResponse(response: response)
    }

    public func parseListWithViewResponse(
        response: WpNetworkResponse
    ) throws -> UsersRequestListWithViewContextResponse {
        try parseAsUsersRequestListWithViewContextResponse(response: response)
    }
}

// MARK: - UsersRequestExecutorProtocol

// Allows the performer to respond to everything the PostsRequestExecutor does
extension UsersRequestPerformer: UsersRequestExecutorProtocol {
    public func create(params: UserCreateParams) async throws -> UsersRequestCreateResponse {
        try await executor.create(params: params)
    }

    public func delete(userId: UserId, params: UserDeleteParams) async throws -> UsersRequestDeleteResponse {
        try await executor.delete(userId: userId, params: params)
    }

    public func deleteMe(params: UserDeleteParams) async throws -> UsersRequestDeleteMeResponse {
        try await executor.deleteMe(params: params)
    }

    public func filterListWithEditContext(
        params: UserListParams,
        fields: [SparseUserFieldWithEditContext]
    ) async throws -> UsersRequestFilterListWithEditContextResponse {
        try await executor.filterListWithEditContext(params: params, fields: fields)
    }

    public func filterListWithEmbedContext(
        params: UserListParams,
        fields: [SparseUserFieldWithEmbedContext]
    ) async throws -> UsersRequestFilterListWithEmbedContextResponse {
        try await executor.filterListWithEmbedContext(params: params, fields: fields)
    }

    public func filterListWithViewContext(
        params: UserListParams,
        fields: [SparseUserFieldWithViewContext]
    ) async throws -> UsersRequestFilterListWithViewContextResponse {
        try await executor.filterListWithViewContext(params: params, fields: fields)
    }

    public func filterRetrieveMeWithEditContext(
        fields: [SparseUserFieldWithEditContext]
    ) async throws -> UsersRequestFilterRetrieveMeWithEditContextResponse {
        try await executor.filterRetrieveMeWithEditContext(fields: fields)
    }

    public func filterRetrieveMeWithEmbedContext(
        fields: [SparseUserFieldWithEmbedContext]
    ) async throws -> UsersRequestFilterRetrieveMeWithEmbedContextResponse {
        try await executor.filterRetrieveMeWithEmbedContext(fields: fields)
    }

    public func filterRetrieveMeWithViewContext(
        fields: [SparseUserFieldWithViewContext]
    ) async throws -> UsersRequestFilterRetrieveMeWithViewContextResponse {
        try await executor.filterRetrieveMeWithViewContext(fields: fields)
    }

    public func filterRetrieveWithEditContext(
        userId: UserId,
        fields: [SparseUserFieldWithEditContext]
    ) async throws -> UsersRequestFilterRetrieveWithEditContextResponse {
        try await executor.filterRetrieveWithEditContext(userId: userId, fields: fields)
    }

    public func filterRetrieveWithEmbedContext(
        userId: UserId,
        fields: [SparseUserFieldWithEmbedContext]
    ) async throws -> UsersRequestFilterRetrieveWithEmbedContextResponse {
        try await executor.filterRetrieveWithEmbedContext(userId: userId, fields: fields)
    }

    public func filterRetrieveWithViewContext(
        userId: UserId,
        fields: [SparseUserFieldWithViewContext]
    ) async throws -> UsersRequestFilterRetrieveWithViewContextResponse {
        try await executor.filterRetrieveWithViewContext(userId: userId, fields: fields)
    }

    public func listWithEditContext(
        params: UserListParams
    ) async throws -> UsersRequestListWithEditContextResponse {
        try await executor.listWithEditContext(params: params)
    }

    public func listWithEmbedContext(
        params: UserListParams
    ) async throws -> UsersRequestListWithEmbedContextResponse {
        try await executor.listWithEmbedContext(params: params)
    }

    public func listWithViewContext(
        params: UserListParams
    ) async throws -> UsersRequestListWithViewContextResponse {
        try await executor.listWithViewContext(params: params)
    }

    public func retrieveMeWithEditContext() async throws -> UsersRequestRetrieveMeWithEditContextResponse {
        try await executor.retrieveMeWithEditContext()
    }

    public func retrieveMeWithEmbedContext() async throws -> UsersRequestRetrieveMeWithEmbedContextResponse {
        try await executor.retrieveMeWithEmbedContext()
    }

    public func retrieveMeWithViewContext() async throws -> UsersRequestRetrieveMeWithViewContextResponse {
        try await executor.retrieveMeWithViewContext()
    }

    public func retrieveWithEditContext(userId: UserId) async throws -> UsersRequestRetrieveWithEditContextResponse {
        try await executor.retrieveWithEditContext(userId: userId)
    }

    public func retrieveWithEmbedContext(userId: UserId) async throws -> UsersRequestRetrieveWithEmbedContextResponse {
        try await executor.retrieveWithEmbedContext(userId: userId)
    }

    public func retrieveWithViewContext(userId: UserId) async throws -> UsersRequestRetrieveWithViewContextResponse {
        try await executor.retrieveWithViewContext(userId: userId)
    }

    public func update(userId: UserId, params: UserUpdateParams) async throws -> UsersRequestUpdateResponse {
        try await executor.update(userId: userId, params: params)
    }

    public func updateMe(params: UserUpdateParams) async throws -> UsersRequestUpdateMeResponse {
        try await executor.updateMe(params: params)
    }
}
