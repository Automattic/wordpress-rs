import Foundation
#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

public protocol Namespace {
    associatedtype T

    var api: WordPressAPI { get }
}

public struct AnyNamespace<T>: Namespace {
    public let api: WordPressAPI
}

public protocol Contextual {
    associatedtype ID
    associatedtype ViewContext
    associatedtype EditContext
    associatedtype EmbedContext

    associatedtype DeleteResult

    associatedtype ListParams
    associatedtype UpdateParams
    associatedtype CreateParams
    associatedtype DeleteParams

    static func retrieveRequest(
        id: ID,
        using requestBuilder: WpRequestBuilderProtocol,
        context: WpContext
    ) -> WpNetworkRequest
    static func listRequest(
        params: ListParams,
        using requestBuilder: WpRequestBuilderProtocol,
        context: WpContext
    ) -> WpNetworkRequest
    static func updateRequest(
        id: ID,
        params: UpdateParams,
        using requestBuilder: WpRequestBuilderProtocol
    ) -> WpNetworkRequest
    static func createRequest(
        params: CreateParams,
        using requestBuilder: WpRequestBuilderProtocol
    ) -> WpNetworkRequest
    static func deleteRequest(
        id: ID,
        params: DeleteParams,
        using requestBuilder: WpRequestBuilderProtocol
    ) -> WpNetworkRequest

    static func parseResponse(_ response: WpNetworkResponse) throws -> ViewContext
    static func parseResponse(_ response: WpNetworkResponse) throws -> EditContext
    static func parseResponse(_ response: WpNetworkResponse) throws -> EmbedContext
    static func parseResponse(_ response: WpNetworkResponse) throws -> [ViewContext]
    static func parseResponse(_ response: WpNetworkResponse) throws -> [EditContext]
    static func parseResponse(_ response: WpNetworkResponse) throws -> [EmbedContext]

    static func parseDeletionResponse(_ response: WpNetworkResponse) throws -> DeleteResult
}

extension AnyNamespace where T: Contextual {
    public var forViewing: ViewNamespace<T> { .init(parent: self) }
    public var forEditing: EditNamespace<T> { .init(parent: self) }
    public var forEmbedding: EmbedNamespace<T> { .init(parent: self) }
}

public protocol ContextualNamespace: Namespace where T: Contextual {
    associatedtype R

    var context: WpContext { get }

    func parseResponse(_ response: WpNetworkResponse) throws -> R
    func parseResponse(_ response: WpNetworkResponse) throws -> [R]
}

public struct ViewNamespace<T: Contextual>: ContextualNamespace {
    public let context: WpContext = .view
    let parent: AnyNamespace<T>

    public var api: WordPressAPI {
        parent.api
    }

    public func parseResponse(_ response: WpNetworkResponse) throws -> T.ViewContext {
        try T.parseResponse(response)
    }

    public func parseResponse(_ response: WpNetworkResponse) throws -> [T.ViewContext] {
        try T.parseResponse(response)
    }
}

public struct EditNamespace<T: Contextual>: ContextualNamespace {
    public let context: WpContext = .edit
    let parent: AnyNamespace<T>

    public var api: WordPressAPI {
        parent.api
    }

    public func parseResponse(_ response: WpNetworkResponse) throws -> T.EditContext {
        try T.parseResponse(response)
    }

    public func parseResponse(_ response: WpNetworkResponse) throws -> [T.EditContext] {
        try T.parseResponse(response)
    }
}

public struct EmbedNamespace<T: Contextual>: ContextualNamespace {
    public let context: WpContext = .embed
    let parent: AnyNamespace<T>

    public var api: WordPressAPI {
        parent.api
    }

    public func parseResponse(_ response: WpNetworkResponse) throws -> T.EmbedContext {
        try T.parseResponse(response)
    }

    public func parseResponse(_ response: WpNetworkResponse) throws -> [T.EmbedContext] {
        try T.parseResponse(response)
    }
}

extension ContextualNamespace {
    public func get(id: T.ID) async throws -> R {
        let request = T.retrieveRequest(id: id, using: api.requestBuilder, context: context)
        let response = try await api.perform(request: request)
        return try parseResponse(response)
    }

    public func list(with params: T.ListParams) async throws -> [R] {
        let request = T.listRequest(params: params, using: api.requestBuilder, context: context)
        let response = try await api.perform(request: request)
        return try parseResponse(response)
    }

    public func list<U>() async throws -> [R] where T.ListParams == U? {
        try await list(with: nil)
    }
}

extension AnyNamespace where T: Contextual {
    public func update(id: T.ID, with params: T.UpdateParams) async throws -> T.EditContext {
        let request = T.updateRequest(id: id, params: params, using: api.requestBuilder)
        let response = try await self.api.perform(request: request)
        return try T.parseResponse(response)
    }

    public func create(using params: T.CreateParams) async throws -> T.EditContext {
        let request = T.createRequest(params: params, using: api.requestBuilder)
        let response = try await self.api.perform(request: request)
        return try T.parseResponse(response)
    }

    public func delete(id: T.ID, params: T.DeleteParams) async throws -> T.DeleteResult {
        let request = T.deleteRequest(id: id, params: params, using: api.requestBuilder)
        let response = try await api.perform(request: request)
        return try T.parseDeletionResponse(response)
    }

    public func delete(id: T.ID) async throws -> T.DeleteResult where T.DeleteParams == Void {
        return try await delete(id: id, params: ())
    }
}
