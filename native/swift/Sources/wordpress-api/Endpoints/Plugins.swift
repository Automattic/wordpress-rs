import Foundation
import WordPressAPIInternal

extension WordPressAPI {
    public var plugins: AnyNamespace<SparsePlugin> {
        .init(api: self)
    }
}

extension SparsePlugin: Contextual {
    public typealias ID = PluginSlug
    public typealias ViewContext = PluginWithViewContext
    public typealias EditContext = PluginWithEditContext
    public typealias EmbedContext = PluginWithEmbedContext

    public static func retrieveRequest(id: PluginSlug, using helper: any WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest {
        helper.retrievePluginRequest(context: context, plugin: id)
    }

    public static func listRequest(params: PluginListParams?, using helper: any WpApiHelperProtocol, context: WpContext) -> WpNetworkRequest {
        helper.listPluginsRequest(context: context, params: params)
    }

    public static func updateRequest(id: PluginSlug, params: PluginUpdateParams, using helper: any WpApiHelperProtocol) -> WpNetworkRequest {
        helper.updatePluginRequest(plugin: id, params: params)
    }

    public static func createRequest(params: PluginCreateParams, using helper: any WpApiHelperProtocol) -> WpNetworkRequest {
        helper.createPluginRequest(params: params)
    }

    public static func deleteRequest(id: PluginSlug, params: Void, using helper: any WpApiHelperProtocol) -> WpNetworkRequest {
        helper.deletePluginRequest(plugin: id)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> PluginWithViewContext {
        try parseRetrievePluginResponseWithViewContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> PluginWithEmbedContext {
        try parseRetrievePluginResponseWithEmbedContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> PluginWithEditContext {
        try parseRetrievePluginResponseWithEditContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> [PluginWithViewContext] {
        try parseListPluginsResponseWithViewContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> [PluginWithEmbedContext] {
        try parseListPluginsResponseWithEmbedContext(response: response)
    }

    public static func parseResponse(_ response: WpNetworkResponse) throws -> [PluginWithEditContext] {
        try parseListPluginsResponseWithEditContext(response: response)
    }

    public static func parseDeletionResponse(_ response: WpNetworkResponse) throws -> PluginDeleteResponse {
        try parseDeletePluginResponse(response: response)
    }
}

// MARK: - Filter

extension ContextualNamespace where T == SparsePlugin {
    public func list(with params: T.ListParams, fields: [SparsePluginField]) async throws -> [T] {
        let request = api.helper.filterListPluginsRequest(context: context, params: params, fields: fields)
        let response = try await api.perform(request: request)
        return try parseFilterPluginsResponse(response: response)
    }

    public func get(id: T.ID, fields: [SparsePluginField]) async throws -> T {
        let request = api.helper.filterRetrievePluginRequest(context: context, plugin: id, fields: fields)
        let response = try await api.perform(request: request)
        return try parseFilterRetrievePluginResponse(response: response)
    }
}

