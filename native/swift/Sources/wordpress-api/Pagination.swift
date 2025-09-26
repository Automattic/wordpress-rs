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

public protocol TypedPaginationAwareExecutor: Sendable {
    associatedtype EditContextResponseType: PaginatableResponse
    associatedtype ViewContextResponseType: PaginatableResponse
    associatedtype EmbedContextResponseType: PaginatableResponse
    associatedtype TypeParam: Sendable

    /// Known function signatures for Request Executors
    func listWithEditContext(
        type: TypeParam,
        params: EditContextResponseType.ParamsType
    ) async throws -> EditContextResponseType

    func listWithViewContext(
        type: TypeParam,
        params: ViewContextResponseType.ParamsType
    ) async throws -> ViewContextResponseType

    func listWithEmbedContext(
        type: TypeParam,
        params: EmbedContextResponseType.ParamsType
    ) async throws -> EmbedContextResponseType

    /// Generated implementation
    func paginatedWithEditContext(
        type: TypeParam,
        params: EditContextResponseType.ParamsType
    ) async throws -> [EditContextResponseType.DataType]

    func paginatedWithViewContext(
        type: TypeParam,
        params: ViewContextResponseType.ParamsType
    ) async throws -> [ViewContextResponseType.DataType]

    func paginatedWithEmbedContext(
        type: TypeParam,
        params: EmbedContextResponseType.ParamsType
    ) async throws -> [EmbedContextResponseType.DataType]

    func sequenceWithEditContext(
        type: TypeParam,
        params: EditContextResponseType.ParamsType
    ) -> PaginationSequence<EditContextResponseType>

    func sequenceWithViewContext(
        type: TypeParam,
        params: ViewContextResponseType.ParamsType
    ) -> PaginationSequence<ViewContextResponseType>

    func sequenceWithEmbedContext(
        type: TypeParam,
        params: EmbedContextResponseType.ParamsType
    ) -> PaginationSequence<EmbedContextResponseType>

}

extension TypedPaginationAwareExecutor {
    /// Fetches all objects from all pages
    ///
    /// This method waits until all objects have been downloaded then returns the results. This can have
    /// unexpected memory and time implications.
    public func paginatedWithEditContext(
        type: TypeParam,
        params: EditContextResponseType.ParamsType
    ) async throws -> [EditContextResponseType.DataType] {
        var allObjects: [EditContextResponseType.DataType] = []
        var mutableParams: EditContextResponseType.ParamsType = params

        repeat {
            let response = try await self.listWithEditContext(type: type, params: mutableParams)
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
        type: TypeParam,
        params: ViewContextResponseType.ParamsType
    ) async throws -> [ViewContextResponseType.DataType] {
        var allObjects: [ViewContextResponseType.DataType] = []
        var mutableParams: ViewContextResponseType.ParamsType = params

        repeat {
            let response = try await self.listWithViewContext(type: type, params: mutableParams)
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
        type: TypeParam,
        params: EmbedContextResponseType.ParamsType
    ) async throws -> [EmbedContextResponseType.DataType] {
        var allObjects: [EmbedContextResponseType.DataType] = []
        var mutableParams: EmbedContextResponseType.ParamsType = params

        repeat {
            let response = try await self.listWithEmbedContext(type: type, params: mutableParams)
            allObjects.append(contentsOf: response.data)

            guard let newParams = response.nextPageParams else {
                break
            }

            mutableParams = newParams
        } while true

        return allObjects
    }

    public func sequenceWithEditContext(
        type: TypeParam,
        params: EditContextResponseType.ParamsType
    ) -> PaginationSequence<EditContextResponseType> {
        PaginationSequence(params: params) { params in
            try await self.listWithEditContext(type: type, params: params)
        }
    }

    public func sequenceWithViewContext(
        type: TypeParam,
        params: ViewContextResponseType.ParamsType
    ) -> PaginationSequence<ViewContextResponseType> {
        PaginationSequence(params: params) { params in
            try await self.listWithViewContext(type: type, params: params)
        }
    }

    public func sequenceWithEmbedContext(
        type: TypeParam,
        params: EmbedContextResponseType.ParamsType
    ) -> PaginationSequence<EmbedContextResponseType> {
        PaginationSequence(params: params) { params in
            try await self.listWithEmbedContext(type: type, params: params)
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
    public typealias DataType = AnyPostWithEditContext
}

extension PostsRequestListWithViewContextResponse: PaginatableResponse {
    public typealias ParamsType = PostListParams
    public typealias DataType = AnyPostWithViewContext
}

extension PostsRequestListWithEmbedContextResponse: PaginatableResponse {
    public typealias ParamsType = PostListParams
    public typealias DataType = AnyPostWithEmbedContext
}

extension PostsRequestExecutor: TypedPaginationAwareExecutor {
    public typealias EditContextResponseType = PostsRequestListWithEditContextResponse
    public typealias ViewContextResponseType = PostsRequestListWithViewContextResponse
    public typealias EmbedContextResponseType = PostsRequestListWithEmbedContextResponse
    public typealias TypeParam = PostEndpointType

    public func listWithEditContext(
        type: TypeParam,
        params: EditContextResponseType.ParamsType
    ) async throws -> EditContextResponseType {
        try await self.listWithEditContext(postEndpointType: type, params: params)
    }

    public func listWithViewContext(
        type: TypeParam,
        params: ViewContextResponseType.ParamsType
    ) async throws -> ViewContextResponseType {
        try await self.listWithViewContext(postEndpointType: type, params: params)
    }

    public func listWithEmbedContext(
        type: TypeParam,
        params: EmbedContextResponseType.ParamsType
    ) async throws -> EmbedContextResponseType {
        try await self.listWithEmbedContext(postEndpointType: type, params: params)
    }
}

// MARK: – Revisions
extension RevisionsRequestListWithEditContextResponse: PaginatableResponse {
    public typealias ParamsType = RevisionListParams
    public typealias DataType = AnyPostRevisionWithEditContext
}

extension RevisionsRequestListWithViewContextResponse: PaginatableResponse {
    public typealias ParamsType = RevisionListParams
    public typealias DataType = AnyPostRevisionWithViewContext
}

extension RevisionsRequestListWithEmbedContextResponse: PaginatableResponse {
    public typealias ParamsType = RevisionListParams
    public typealias DataType = AnyPostRevisionWithEmbedContext
}

extension RevisionsRequestExecutor {
    public typealias EditContextResponseType = RevisionsRequestListWithEditContextResponse
    public typealias ViewContextResponseType = RevisionsRequestListWithViewContextResponse
    public typealias EmbedContextResponseType = RevisionsRequestListWithEmbedContextResponse
    public typealias TypeParam = PostEndpointType

    public func listWithEditContext(
        type: TypeParam,
        postId: PostId,
        params: EditContextResponseType.ParamsType
    ) async throws -> EditContextResponseType {
        try await self.listWithEditContext(postEndpointType: type, postId: postId, params: params)
    }

    public func listWithViewContext(
        type: TypeParam,
        postId: PostId,
        params: ViewContextResponseType.ParamsType
    ) async throws -> ViewContextResponseType {
        try await self.listWithViewContext(postEndpointType: type, postId: postId, params: params)
    }

    public func listWithEmbedContext(
        type: TypeParam,
        postId: PostId,
        params: EmbedContextResponseType.ParamsType
    ) async throws -> EmbedContextResponseType {
        try await self.listWithEmbedContext(postEndpointType: type, postId: postId, params: params)
    }

    public func sequenceWithEditContext(
        type: TypeParam,
        postId: PostId,
        params: EditContextResponseType.ParamsType
    ) -> PaginationSequence<EditContextResponseType> {
        PaginationSequence(params: params) { params in
            try await self.listWithEditContext(type: type, postId: postId, params: params)
        }
    }

    public func sequenceWithViewContext(
        type: TypeParam,
        postId: PostId,
        params: ViewContextResponseType.ParamsType
    ) -> PaginationSequence<ViewContextResponseType> {
        PaginationSequence(params: params) { params in
            try await self.listWithViewContext(type: type, postId: postId, params: params)
        }
    }

    public func sequenceWithEmbedContext(
        type: TypeParam,
        postId: PostId,
        params: EmbedContextResponseType.ParamsType
    ) -> PaginationSequence<EmbedContextResponseType> {
        PaginationSequence(params: params) { params in
            try await self.listWithEmbedContext(type: type, postId: postId, params: params)
        }
    }
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

// MARK: - Terms
extension TermsRequestListWithEditContextResponse: PaginatableResponse {
    public typealias ParamsType = TermListParams
    public typealias DataType = AnyTermWithEditContext
}

extension TermsRequestListWithViewContextResponse: PaginatableResponse {
    public typealias ParamsType = TermListParams
    public typealias DataType = AnyTermWithViewContext
}

extension TermsRequestListWithEmbedContextResponse: PaginatableResponse {
    public typealias ParamsType = TermListParams
    public typealias DataType = AnyTermWithEmbedContext
}

extension TermsRequestExecutor: TypedPaginationAwareExecutor {
    public typealias EditContextResponseType = TermsRequestListWithEditContextResponse
    public typealias ViewContextResponseType = TermsRequestListWithViewContextResponse
    public typealias EmbedContextResponseType = TermsRequestListWithEmbedContextResponse
    public typealias TypeParam = TermEndpointType

    public func listWithEditContext(
        type: TypeParam,
        params: EditContextResponseType.ParamsType
    ) async throws -> EditContextResponseType {
        try await self.listWithEditContext(termEndpointType: type, params: params)
    }

    public func listWithViewContext(
        type: TypeParam,
        params: ViewContextResponseType.ParamsType
    ) async throws -> ViewContextResponseType {
        try await self.listWithViewContext(termEndpointType: type, params: params)
    }

    public func listWithEmbedContext(
        type: TypeParam,
        params: EmbedContextResponseType.ParamsType
    ) async throws -> EmbedContextResponseType {
        try await self.listWithEmbedContext(termEndpointType: type, params: params)
    }
}
