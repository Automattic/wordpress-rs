import Foundation
import wordpress_api_wrapper

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

    static func makeGetOneRequest(id: ID, using helper: WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest
    static func makeGetListRequest(using helper: WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest
    static func parseResponse(_ response: WpNetworkResponse) throws -> ViewContext
    static func parseResponse(_ response: WpNetworkResponse) throws -> EditContext
    static func parseResponse(_ response: WpNetworkResponse) throws -> EmbedContext
    static func parseResponse(_ response: WpNetworkResponse) throws -> [ViewContext]
    static func parseResponse(_ response: WpNetworkResponse) throws -> [EditContext]
    static func parseResponse(_ response: WpNetworkResponse) throws -> [EmbedContext]
}

extension AnyNamespace where T: Contextual {
    public var forViewing: ViewNamespace<T> { .init(parent: self) }
    public var forEditing: EditNamespace<T> { .init(parent: self) }
    public var forEmbedding: EmbedNamespace<T> { .init(parent: self) }
}

public protocol ContextualNamespace: Namespace where T: Contextual {
    associatedtype R

    var context: WpContext { get }

    func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> R
    func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> [R]
}

public struct ViewNamespace<T: Contextual>: ContextualNamespace {
    public let context: WpContext = .view
    let parent: AnyNamespace<T>

    public var api: WordPressAPI {
        parent.api
    }

    public func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> T.ViewContext {
        try T.parseResponse(response)
    }

    public func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> [T.ViewContext] {
        try T.parseResponse(response)
    }
}

public struct EditNamespace<T: Contextual>: ContextualNamespace {
    public let context: WpContext = .edit
    let parent: AnyNamespace<T>

    public var api: WordPressAPI {
        parent.api
    }

    public func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> T.EditContext {
        try T.parseResponse(response)
    }

    public func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> [T.EditContext] {
        try T.parseResponse(response)
    }
}

public struct EmbedNamespace<T: Contextual>: ContextualNamespace {
    public let context: WpContext = .embed
    let parent: AnyNamespace<T>

    public var api: WordPressAPI {
        parent.api
    }

    public func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> T.EmbedContext {
        try T.parseResponse(response)
    }

    public func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> [T.EmbedContext] {
        try T.parseResponse(response)
    }
}

extension ContextualNamespace {
    public func get(id: T.ID) async throws -> R {
        let request = T.makeGetOneRequest(id: id, using: api.helper, context: context)
        let response = try await api.perform(request: request)
        return try parseResponse(response)
    }

    public func list() async throws -> [R] {
        let request = T.makeGetListRequest(using: api.helper, context: context)
        let response = try await api.perform(request: request)
        return try parseResponse(response)
    }
}
