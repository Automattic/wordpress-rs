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
    associatedtype View
    associatedtype Edit
    associatedtype Embed

    static func makeGetOneRequest(id: ID, using helper: WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest
    static func makeGetListRequest(using helper: WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest
    static func parseResponse(_ response: WpNetworkResponse) throws -> View
    static func parseResponse(_ response: WpNetworkResponse) throws -> Edit
    static func parseResponse(_ response: WpNetworkResponse) throws -> Embed
    static func parseResponse(_ response: WpNetworkResponse) throws -> [View]
    static func parseResponse(_ response: WpNetworkResponse) throws -> [Edit]
    static func parseResponse(_ response: WpNetworkResponse) throws -> [Embed]
}

extension AnyNamespace where T: Contextual {
    public var forViewing: ViewNamespace<T> { .init(parent: self) }
    public var forEditing: EditNamespace<T> { .init(parent: self) }
    public var forEmbedding: EmbedNamespace<T> { .init(parent: self) }
}

public protocol ContextualNamespace: Namespace where T: Contextual {
    associatedtype R

    func makeGetOneRequest(id: T.ID, using helper: any wordpress_api_wrapper.WpApiHelperProtocol) -> wordpress_api_wrapper.WpNetworkRequest
    func makeGetListRequest(using helper: any wordpress_api_wrapper.WpApiHelperProtocol) -> wordpress_api_wrapper.WpNetworkRequest
    func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> R
    func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> [R]
}

public struct ViewNamespace<T: Contextual>: ContextualNamespace {
    let parent: AnyNamespace<T>

    public var api: WordPressAPI {
        parent.api
    }

    public func makeGetOneRequest(id: T.ID, using helper: any wordpress_api_wrapper.WpApiHelperProtocol) -> wordpress_api_wrapper.WpNetworkRequest {
        T.makeGetOneRequest(id: id, using: helper, context: .view)
    }

    public func makeGetListRequest(using helper: any wordpress_api_wrapper.WpApiHelperProtocol) -> wordpress_api_wrapper.WpNetworkRequest {
        T.makeGetListRequest(using: helper, context: .view)
    }

    public func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> T.View {
        try T.parseResponse(response)
    }

    public func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> [T.View] {
        try T.parseResponse(response)
    }
}

public struct EditNamespace<T: Contextual>: ContextualNamespace {
    let parent: AnyNamespace<T>

    public var api: WordPressAPI {
        parent.api
    }
    public func makeGetOneRequest(id: T.ID, using helper: any wordpress_api_wrapper.WpApiHelperProtocol) -> wordpress_api_wrapper.WpNetworkRequest {
        T.makeGetOneRequest(id: id, using: helper, context: .edit)
    }

    public func makeGetListRequest(using helper: any wordpress_api_wrapper.WpApiHelperProtocol) -> wordpress_api_wrapper.WpNetworkRequest {
        T.makeGetListRequest(using: helper, context: .edit)
    }

    public func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> T.Edit {
        try T.parseResponse(response)
    }

    public func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> [T.Edit] {
        try T.parseResponse(response)
    }
}

public struct EmbedNamespace<T: Contextual>: ContextualNamespace {
    let parent: AnyNamespace<T>

    public var api: WordPressAPI {
        parent.api
    }
    public func makeGetOneRequest(id: T.ID, using helper: any wordpress_api_wrapper.WpApiHelperProtocol) -> wordpress_api_wrapper.WpNetworkRequest {
        T.makeGetOneRequest(id: id, using: helper, context: .embed)
    }

    public func makeGetListRequest(using helper: any wordpress_api_wrapper.WpApiHelperProtocol) -> wordpress_api_wrapper.WpNetworkRequest {
        T.makeGetListRequest(using: helper, context: .embed)
    }

    public func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> T.Embed {
        try T.parseResponse(response)
    }

    public func parseResponse(_ response: wordpress_api_wrapper.WpNetworkResponse) throws -> [T.Embed] {
        try T.parseResponse(response)
    }
}

extension ContextualNamespace {
    public func get(id: T.ID) async throws -> R {
        let request = makeGetOneRequest(id: id, using: api.helper)
        let response = try await api.perform(request: request)
        return try parseResponse(response)
    }

    public func list() async throws -> [R] {
        let request = makeGetListRequest(using: api.helper)
        let response = try await api.perform(request: request)
        return try parseResponse(response)
    }
}
