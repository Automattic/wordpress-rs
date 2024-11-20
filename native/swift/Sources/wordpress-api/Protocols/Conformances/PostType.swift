import Foundation

#if canImport(WordPressAPIInternal)
@preconcurrency import WordPressAPIInternal
#endif

public final class PostTypeRequestPerformer {
    typealias ExecutorType = PostTypesRequestExecutor
    typealias RequestBuilderType = PostTypesRequestBuilder

    let executor: ExecutorType
    let builder: RequestBuilderType
    public let urlSession: URLSession

    init(executor: ExecutorType, builder: RequestBuilderType, session: URLSession) {
        self.executor = executor
        self.builder = builder
        self.urlSession = session
    }
}

extension PostTypeRequestPerformer: PostTypesRequestExecutorProtocol {
    public func filterRetrieveWithEditContext(
        postType: PostType,
        fields: [SparsePostTypeDetailsFieldWithEditContext]
    ) async throws -> WordPressAPIInternal.PostTypesRequestFilterRetrieveWithEditContextResponse {
        try await executor.filterRetrieveWithEditContext(postType: postType, fields: fields)
    }

    public func filterRetrieveWithEmbedContext(
        postType: PostType,
        fields: [SparsePostTypeDetailsFieldWithEmbedContext]
    ) async throws -> PostTypesRequestFilterRetrieveWithEmbedContextResponse {
        try await executor.filterRetrieveWithEmbedContext(postType: postType, fields: fields)
    }

    public func filterRetrieveWithViewContext(
        postType: PostType,
        fields: [SparsePostTypeDetailsFieldWithViewContext]
    ) async throws -> PostTypesRequestFilterRetrieveWithViewContextResponse {
        try await executor.filterRetrieveWithViewContext(postType: postType, fields: fields)
    }

    public func listWithEditContext() async throws -> PostTypesRequestListWithEditContextResponse {
        try await executor.listWithEditContext()
    }

    public func listWithEmbedContext() async throws -> PostTypesRequestListWithEmbedContextResponse {
        try await executor.listWithEmbedContext()
    }

    public func listWithViewContext() async throws -> PostTypesRequestListWithViewContextResponse {
        try await executor.listWithViewContext()
    }

    public func retrieveWithEditContext(
        postType: PostType
    ) async throws -> PostTypesRequestRetrieveWithEditContextResponse {
        try await executor.retrieveWithEditContext(postType: postType)
    }

    public func retrieveWithEmbedContext(
        postType: PostType
    ) async throws -> PostTypesRequestRetrieveWithEmbedContextResponse {
        try await executor.retrieveWithEmbedContext(postType: postType)
    }

    public func retrieveWithViewContext(
        postType: PostType
    ) async throws -> PostTypesRequestRetrieveWithViewContextResponse {
        try await executor.retrieveWithViewContext(postType: postType)
    }
}
