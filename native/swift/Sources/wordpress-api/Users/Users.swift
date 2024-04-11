import Foundation
import wordpress_api_wrapper

extension SparseUser: Contextual {}

extension WordPressAPI {
    public var users: AnyNamespace<SparseUser> {
        .init(api: self)
    }
}

extension ViewNamespace where T == SparseUser {

    public func get(id: T.ID) async throws -> T.View {
        let request = self.api.helper.retrieveUserRequest(userId: id, context: .view)
        let response = try await api.perform(request: request)
        return try parseRetrieveUserResponseWithViewContext(response: response)
    }

    public func getCurrent() async throws -> T.View {
        let request = self.api.helper.retrieveCurrentUserRequest(context: .view)
        let response = try await api.perform(request: request)
        return try parseRetrieveUserResponseWithViewContext(response: response)
    }

    public func list() async throws -> [T.View] {
        let request = self.api.helper.listUsersRequest(context: .view, params: nil)
        let response = try await api.perform(request: request)
        return try parseListUsersResponseWithViewContext(response: response)
    }

}

extension EditNamespace where T == SparseUser {

    public func get(id: T.ID) async throws -> T.Edit {
        let request = self.api.helper.retrieveUserRequest(userId: id, context: .edit)
        let response = try await api.perform(request: request)
        return try parseRetrieveUserResponseWithEditContext(response: response)
    }

    public func getCurrent() async throws -> T.Edit {
        let request = self.api.helper.retrieveCurrentUserRequest(context: .edit)
        let response = try await api.perform(request: request)
        return try parseRetrieveUserResponseWithEditContext(response: response)
    }

    public func list() async throws -> [T.Edit] {
        let request = self.api.helper.listUsersRequest(context: .edit, params: nil)
        let response = try await api.perform(request: request)
        return try parseListUsersResponseWithEditContext(response: response)
    }

}

extension EmbedNamespace where T == SparseUser {

    public func get(id: T.ID) async throws -> T.Embed {
        let request = self.api.helper.retrieveUserRequest(userId: id, context: .embed)
        let response = try await api.perform(request: request)
        return try parseRetrieveUserResponseWithEmbedContext(response: response)
    }

    public func getCurrent() async throws -> T.Embed {
        let request = self.api.helper.retrieveCurrentUserRequest(context: .embed)
        let response = try await api.perform(request: request)
        return try parseRetrieveUserResponseWithEmbedContext(response: response)
    }

    public func list() async throws -> [T.Embed] {
        let request = self.api.helper.listUsersRequest(context: .embed, params: nil)
        let response = try await api.perform(request: request)
        return try parseListUsersResponseWithEmbedContext(response: response)
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

    public func update(id: T.ID, with params: UserUpdateParams) async throws -> T.Edit {
        let request = self.api.helper.updateUserRequest(userId: id, params: params)
        let response = try await self.api.perform(request: request)
        return try parseRetrieveUserResponseWithEditContext(response: response)
    }

    public func create(using params: UserCreateParams) async throws -> T.Edit {
        let request = self.api.helper.createUserRequest(params: params)
        let response = try await self.api.perform(request: request)
        return try parseRetrieveUserResponseWithEditContext(response: response)
    }

    public func deleteCurrent(reassignTo userID: T.ID) async throws {
        let request = self.api.helper.deleteCurrentUserRequest(params: .init(reassign: userID))
        let response = try await api.perform(request: request)
        // TODO: Parse response to check if there is any error
        return
    }

    public func updateCurrent(with params: UserUpdateParams) async throws -> T.Edit {
        let request = self.api.helper.updateCurrentUserRequest(params: params)
        let response = try await self.api.perform(request: request)
        return try parseRetrieveUserResponseWithEditContext(response: response)
    }

}
