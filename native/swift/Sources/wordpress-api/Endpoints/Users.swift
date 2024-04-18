import Foundation
import wordpress_api_wrapper

extension SparseUser: Contextual {
    public typealias ID = UserId
    public typealias ViewContext = UserWithViewContext
    public typealias EditContext = UserWithEditContext
    public typealias EmbedContext = UserWithEmbedContext

    public static func retrieveRequest(id: UserId, using helper: WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest {
        helper.retrieveUserRequest(userId: id, context: context)
    }

    public static func listRequest(using helper: WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest {
        helper.listUsersRequest(context: context, params: nil)
    }

    public static func updateRequest(id: PostId, params: UserUpdateParams, using helper: any WpApiHelperProtocol) -> WpNetworkRequest {
        helper.updateUserRequest(userId: id, params: params)
    }

    public static func createRequest(params: UserCreateParams, using helper: any WpApiHelperProtocol) -> WpNetworkRequest {
        helper.createUserRequest(params: params)
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

    public func delete(id: T.ID, reassignTo userID: T.ID) async throws {
        let request = self.api.helper.deleteUserRequest(userId: id, params: .init(reassign: userID))
        let response = try await api.perform(request: request)
        // TODO: Missing parse response
        return
    }

    public func deleteCurrent(reassignTo userID: T.ID) async throws {
        let request = self.api.helper.deleteCurrentUserRequest(params: .init(reassign: userID))
        let response = try await api.perform(request: request)
        // TODO: Parse response to check if there is any error
        return
    }

    public func updateCurrent(with params: UserUpdateParams) async throws -> T.EditContext {
        let request = self.api.helper.updateCurrentUserRequest(params: params)
        let response = try await self.api.perform(request: request)
        return try parseRetrieveUserResponseWithEditContext(response: response)
    }

}
