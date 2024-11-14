import Foundation

public protocol PaginatableResponse {
    associatedtype ParamsType
    associatedtype DataType

    var nextPageParams: ParamsType? { get }
    var prevPageParams: ParamsType? { get }

    var data: [DataType] { get }
}

public protocol PaginationAwareExecutor {
    associatedtype EditContextResponseType: PaginatableResponse
    associatedtype ViewContextResponseType: PaginatableResponse
    associatedtype EmbedContextResponseType: PaginatableResponse

    /// Known function signatures for Request Executors
    func listWithEditContext(params: EditContextResponseType.ParamsType) async throws -> EditContextResponseType
    func listWithViewContext(params: ViewContextResponseType.ParamsType) async throws -> ViewContextResponseType
    func listWithEmbedContext(params: EmbedContextResponseType.ParamsType) async throws -> EmbedContextResponseType

    /// Generated implementation
    func paginatedWithEditContext(
        params: EditContextResponseType.ParamsType
    ) async throws -> [EditContextResponseType.DataType]

    func paginatedWithViewContext(
        params: ViewContextResponseType.ParamsType
    ) async throws -> [ViewContextResponseType.DataType]

    func paginatedWithEmbedContext(
        params: EmbedContextResponseType.ParamsType
    ) async throws -> [EmbedContextResponseType.DataType]
}

extension PaginationAwareExecutor {
    /// Fetches all objects from all pages
    ///
    /// This method waits until all objects have been downloaded then returns the results. This can have
    /// unexpected memory and time implications.
    public func paginatedWithEditContext(
        params: EditContextResponseType.ParamsType
    ) async throws -> [EditContextResponseType.DataType] {
        var allObjects: [EditContextResponseType.DataType] = []
        var mutableParams: EditContextResponseType.ParamsType = params

        repeat {
            let response = try await self.listWithEditContext(params: mutableParams)
            allObjects.append(contentsOf: response.data)

            guard let newParams = response.nextPageParams else {
                break
            }

            mutableParams = newParams
        } while true

        return allObjects
    }

    /// Fetches all objects from all pages
    ///
    /// This method waits until all objects have been downloaded then returns the results. This can have
    /// unexpected memory and time implications.
    public func paginatedWithViewContext(
        params: ViewContextResponseType.ParamsType
    ) async throws -> [ViewContextResponseType.DataType] {
        var allObjects: [ViewContextResponseType.DataType] = []
        var mutableParams: ViewContextResponseType.ParamsType = params

        repeat {
            let response = try await self.listWithViewContext(params: mutableParams)
            allObjects.append(contentsOf: response.data)

            guard let newParams = response.nextPageParams else {
                break
            }

            mutableParams = newParams
        } while true

        return allObjects
    }

    /// Fetches all objects from all pages
    ///
    /// This method waits until all objects have been downloaded then returns the results. This can have
    /// unexpected memory and time implications.
    public func paginatedWithEmbedContext(
        params: EmbedContextResponseType.ParamsType
    ) async throws -> [EmbedContextResponseType.DataType] {
        var allObjects: [EmbedContextResponseType.DataType] = []
        var mutableParams: EmbedContextResponseType.ParamsType = params

        repeat {
            let response = try await self.listWithEmbedContext(params: mutableParams)
            allObjects.append(contentsOf: response.data)

            guard let newParams = response.nextPageParams else {
                break
            }

            mutableParams = newParams
        } while true

        return allObjects
    }
}

// MARK: - Posts
extension PostsRequestListWithEditContextResponse: PaginatableResponse {
    public typealias ParamsType = PostListParams
    public typealias DataType = PostWithEditContext
}

extension PostsRequestListWithViewContextResponse: PaginatableResponse {
    public typealias ParamsType = PostListParams
    public typealias DataType = PostWithViewContext
}

extension PostsRequestListWithEmbedContextResponse: PaginatableResponse {
    public typealias ParamsType = PostListParams
    public typealias DataType = PostWithEmbedContext
}

extension PostsRequestExecutor: PaginationAwareExecutor {
    public typealias EditContextResponseType = PostsRequestListWithEditContextResponse
    public typealias ViewContextResponseType = PostsRequestListWithViewContextResponse
    public typealias EmbedContextResponseType = PostsRequestListWithEmbedContextResponse
}

// MARK: - Users
extension UsersRequestListWithEditContextResponse: PaginatableResponse {
    public typealias ParamsType = UserListParams
    public typealias DataType = UserWithEditContext
}

extension UsersRequestListWithViewContextResponse: PaginatableResponse {
    public typealias ParamsType = UserListParams
    public typealias DataType = UserWithViewContext
}

extension UsersRequestListWithEmbedContextResponse: PaginatableResponse {
    public typealias ParamsType = UserListParams
    public typealias DataType = UserWithEmbedContext
}

extension UsersRequestExecutor: PaginationAwareExecutor {
    public typealias EditContextResponseType = UsersRequestListWithEditContextResponse
    public typealias ViewContextResponseType = UsersRequestListWithViewContextResponse
    public typealias EmbedContextResponseType = UsersRequestListWithEmbedContextResponse
}
