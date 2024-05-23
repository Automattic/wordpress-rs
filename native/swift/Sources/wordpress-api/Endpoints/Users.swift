import Foundation
import WordPressAPIInternal

extension SparseUser: Contextual {
    public typealias ID = UserId
    public typealias ViewContext = UserWithViewContext
    public typealias EditContext = UserWithEditContext
    public typealias EmbedContext = UserWithEmbedContext

    public static func retrieveRequest(id: UserId, using helper: WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest {
        helper.retrieveUserRequest(userId: id, context: context)
    }

    public static func listRequest(params: UserListParams?, using helper: WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest {
        helper.listUsersRequest(context: context, params: params)
    }

    public static func updateRequest(id: UserId, params: UserUpdateParams, using helper: any WpApiHelperProtocol) -> WpNetworkRequest {
        helper.updateUserRequest(userId: id, params: params)
    }

    public static func createRequest(params: UserCreateParams, using helper: any WpApiHelperProtocol) -> WpNetworkRequest {
        helper.createUserRequest(params: params)
    }

    public static func deleteRequest(id: ID, params: UserDeleteParams, using helper: WpApiHelperProtocol) -> WpNetworkRequest {
        helper.deleteUserRequest(userId: id, params: params)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> UserWithViewContext {
        try parseRetrieveUserResponseWithViewContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> UserWithEditContext {
        try parseRetrieveUserResponseWithEditContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> UserWithEmbedContext {
        try parseRetrieveUserResponseWithEmbedContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> [UserWithViewContext] {
        try parseListUsersResponseWithViewContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> [UserWithEditContext] {
        try parseListUsersResponseWithEditContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> [UserWithEmbedContext] {
        try parseListUsersResponseWithEmbedContext(response: response)
    }

    public static func parseDeletionResponse(_ response: WpNetworkResponse) throws -> UserDeleteResponse {
        try parseDeleteUserResponse(response: response)
    }
}

extension WordPressAPI {
    public var users: AnyNamespace<SparseUser> {
        .init(api: self)
    }
}

extension ContextualNamespace where T == SparseUser {
    public func getCurrent() async throws -> R {
        let request = self.api.helper.retrieveCurrentUserRequest(context: context)
        let response = try await api.perform(request: request)
        return try parseResponse(response)
    }
}

// MARK: - Edit context

extension AnyNamespace where T == SparseUser {

    public func delete(id: T.ID, reassignTo userID: T.ID) async throws -> T.DeleteResult {
        try await self.delete(id: id, params: .init(reassign: userID))
    }

    public func deleteCurrent(reassignTo userID: T.ID) async throws -> T.DeleteResult {
        let request = self.api.helper.deleteCurrentUserRequest(params: .init(reassign: userID))
        let response = try await api.perform(request: request)
        return try T.parseDeletionResponse(response)
    }

    public func updateCurrent(with params: UserUpdateParams) async throws -> T.EditContext {
        let request = self.api.helper.updateCurrentUserRequest(params: params)
        let response = try await self.api.perform(request: request)
        return try parseRetrieveUserResponseWithEditContext(response: response)
    }

}

// MARK: - Filter

// Note: Once the Rust library adds support of passing fields to all routes (get, list, update, create, etc), we can
// consider generalizing these functions and moving them to Namespace.swift.
extension ContextualNamespace where T == SparseUser {

    public func list(with params: T.ListParams, fields: [SparseUserField]) async throws -> [T] {
        let request = api.helper.filterListUsersRequest(context: context, params: params, fields: fields)
        let response = try await api.perform(request: request)
        return try parseFilterUsersResponse(response: response)
    }

    public func get(id: T.ID, fields: [SparseUserField]) async throws -> T {
        let request = api.helper.filterRetrieveUserRequest(userId: id, context: context, fields: fields)
        let response = try await api.perform(request: request)
        return try parseFilterRetrieveUserResponse(response: response)
    }

    public func getCurrent(fields: [SparseUserField]) async throws -> T {
        let request = api.helper.filterRetrieveCurrentUserRequest(context: context, fields: fields)
        let response = try await api.perform(request: request)
        return try parseFilterRetrieveUserResponse(response: response)
    }

}