import Foundation
import WordPressAPIInternal

public protocol PaginatableResponse: Sendable {
    associatedtype ParamsType: Sendable
    associatedtype DataType: Sendable

    var nextPageParams: ParamsType? { get }
    var prevPageParams: ParamsType? { get }

    var data: [DataType] { get }

    init(data: [DataType], headerMap: WpNetworkHeaderMap, nextPageParams: ParamsType?, prevPageParams: ParamsType?)
}

public protocol PaginationAwareExecutor: Sendable {
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

    func sequenceWithEditContext(
        params: EditContextResponseType.ParamsType
    ) -> PaginationSequence<EditContextResponseType>

    func sequenceWithViewContext(
        params: ViewContextResponseType.ParamsType
    ) -> PaginationSequence<ViewContextResponseType>

    func sequenceWithEmbedContext(
        params: EmbedContextResponseType.ParamsType
    ) -> PaginationSequence<EmbedContextResponseType>
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

    public func sequenceWithEditContext(
        params: EditContextResponseType.ParamsType
    ) -> PaginationSequence<EditContextResponseType> {
        PaginationSequence(params: params) { params in
            try await self.listWithEditContext(params: params)
        }
    }

    public func sequenceWithViewContext(
        params: ViewContextResponseType.ParamsType
    ) -> PaginationSequence<ViewContextResponseType> {
        PaginationSequence(params: params) { params in
            try await self.listWithViewContext(params: params)
        }
    }

    public func sequenceWithEmbedContext(
        params: EmbedContextResponseType.ParamsType
    ) -> PaginationSequence<EmbedContextResponseType> {
        PaginationSequence(params: params) { params in
            try await self.listWithEmbedContext(params: params)
        }
    }
}

public struct PaginationSequence<ResponseType: PaginatableResponse>: AsyncSequence, Sendable {
    public typealias Transformer = @Sendable (ResponseType.ParamsType) async throws -> ResponseType

    private let params: ResponseType.ParamsType
    private let transform: Transformer

    init(params: ResponseType.ParamsType, transform: @escaping Transformer) {
        self.params = params
        self.transform = transform
    }

    public struct AsyncIterator: AsyncIteratorProtocol {
        private var nextPageParams: ResponseType.ParamsType?
        private let transform: Transformer

        init(params: ResponseType.ParamsType, transform: @escaping Transformer) {
            self.nextPageParams = params
            self.transform = transform
        }

        public mutating func next() async throws -> [ResponseType.DataType]? {
            guard let nextPageParams else {
                return nil
            }

            let response = try await self.transform(nextPageParams)
            self.nextPageParams = response.nextPageParams
            return response.data
        }
    }

    public func makeAsyncIterator() -> AsyncIterator {
        AsyncIterator(params: params, transform: self.transform)
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

// MARK: - Pages
extension PagesRequestListWithEditContextResponse: PaginatableResponse {
    public typealias ParamsType = PageListParams
    public typealias DataType = PageWithEditContext
}

extension PagesRequestListWithViewContextResponse: PaginatableResponse {
    public typealias ParamsType = PageListParams
    public typealias DataType = PageWithViewContext
}

extension PagesRequestListWithEmbedContextResponse: PaginatableResponse {
    public typealias ParamsType = PageListParams
    public typealias DataType = PageWithEmbedContext
}

extension PagesRequestExecutor: PaginationAwareExecutor {
    public typealias EditContextResponseType = PagesRequestListWithEditContextResponse
    public typealias ViewContextResponseType = PagesRequestListWithViewContextResponse
    public typealias EmbedContextResponseType = PagesRequestListWithEmbedContextResponse
}

// MARK: - Media
extension MediaRequestListWithEditContextResponse: PaginatableResponse {
    public typealias ParamsType = MediaListParams
    public typealias DataType = MediaWithEditContext
}

extension MediaRequestListWithViewContextResponse: PaginatableResponse {
    public typealias ParamsType = MediaListParams
    public typealias DataType = MediaWithViewContext
}

extension MediaRequestListWithEmbedContextResponse: PaginatableResponse {
    public typealias ParamsType = MediaListParams
    public typealias DataType = MediaWithEmbedContext
}

extension MediaRequestExecutor: PaginationAwareExecutor {
    public typealias EditContextResponseType = MediaRequestListWithEditContextResponse
    public typealias ViewContextResponseType = MediaRequestListWithViewContextResponse
    public typealias EmbedContextResponseType = MediaRequestListWithEmbedContextResponse
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

// MARK: - Comments
extension CommentsRequestListWithEditContextResponse: PaginatableResponse {
    public typealias ParamsType = CommentListParams
    public typealias DataType = CommentWithEditContext
}

extension CommentsRequestListWithViewContextResponse: PaginatableResponse {
    public typealias ParamsType = CommentListParams
    public typealias DataType = CommentWithViewContext
}

extension CommentsRequestListWithEmbedContextResponse: PaginatableResponse {
    public typealias ParamsType = CommentListParams
    public typealias DataType = CommentWithEmbedContext
}

extension CommentsRequestExecutor: PaginationAwareExecutor {
    public typealias EditContextResponseType = CommentsRequestListWithEditContextResponse
    public typealias ViewContextResponseType = CommentsRequestListWithViewContextResponse
    public typealias EmbedContextResponseType = CommentsRequestListWithEmbedContextResponse
}

// MARK: - Categories

extension CategoriesRequestListWithEditContextResponse: PaginatableResponse {
    public typealias ParamsType = CategoryListParams
    public typealias DataType = CategoryWithEditContext
}

extension CategoriesRequestListWithViewContextResponse: PaginatableResponse {
    public typealias ParamsType = CategoryListParams
    public typealias DataType = CategoryWithViewContext
}

extension CategoriesRequestListWithEmbedContextResponse: PaginatableResponse {
    public typealias ParamsType = CategoryListParams
    public typealias DataType = CategoryWithEmbedContext
}

extension CategoriesRequestExecutor: PaginationAwareExecutor {
    public typealias EditContextResponseType = CategoriesRequestListWithEditContextResponse
    public typealias ViewContextResponseType = CategoriesRequestListWithViewContextResponse
    public typealias EmbedContextResponseType = CategoriesRequestListWithEmbedContextResponse
}

// MARK: - Tags

extension TagsRequestListWithEditContextResponse: PaginatableResponse {
    public typealias ParamsType = TagListParams
    public typealias DataType = TagWithEditContext
}

extension TagsRequestListWithViewContextResponse: PaginatableResponse {
    public typealias ParamsType = TagListParams
    public typealias DataType = TagWithViewContext
}

extension TagsRequestListWithEmbedContextResponse: PaginatableResponse {
    public typealias ParamsType = TagListParams
    public typealias DataType = TagWithEmbedContext
}

extension TagsRequestExecutor: PaginationAwareExecutor {
    public typealias EditContextResponseType = TagsRequestListWithEditContextResponse
    public typealias ViewContextResponseType = TagsRequestListWithViewContextResponse
    public typealias EmbedContextResponseType = TagsRequestListWithEmbedContextResponse
}
