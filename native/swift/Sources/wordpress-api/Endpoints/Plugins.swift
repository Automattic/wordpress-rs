import Foundation
#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

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

    public static func retrieveRequest(id: PluginSlug, using requestBuilder: any WpRequestBuilderProtocol, context: WpContext) -> WpNetworkRequest {
        requestBuilder.plugins().retrieve(context: context, plugin: id)
    }

    public static func listRequest(params: PluginListParams?, using requestBuilder: any WpRequestBuilderProtocol, context: WpContext) -> WpNetworkRequest {
        requestBuilder.plugins().list(context: context, params: params)
    }

    public static func updateRequest(id: PluginSlug, params: PluginUpdateParams, using requestBuilder: any WpRequestBuilderProtocol) -> WpNetworkRequest {
        requestBuilder.plugins().update(plugin: id, params: params)
    }

    public static func createRequest(params: PluginCreateParams, using requestBuilder: any WpRequestBuilderProtocol) -> WpNetworkRequest {
        requestBuilder.plugins().create(params: params)
    }

    public static func deleteRequest(id: PluginSlug, params: Void, using requestBuilder: any WpRequestBuilderProtocol) -> WpNetworkRequest {
        requestBuilder.plugins().delete(plugin: id)
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

// MARK: - Convenience Methods

public extension AnyNamespace where T == SparsePlugin {
    func install(slug: PluginWpOrgDirectorySlug, status: PluginStatus) async throws -> PluginWithEditContext {
        try await create(using: .init(slug: slug, status: status))
    }

    func uninstall(slug: PluginSlug) async throws -> PluginDeleteResponse {
        try await delete(id: slug)
    }

    func activate(slug: PluginSlug) async throws -> PluginWithEditContext {
        // We don't support activate the plugin on all sites in the network yet.
        try await update(id: slug, with: .init(status: .active))
    }

    func deactivate(slug: PluginSlug) async throws -> PluginWithEditContext {
        try await update(id: slug, with: .init(status: .inactive))
    }
}

// MARK: - Filter

extension ContextualNamespace where T == SparsePlugin {
    public func list(with params: T.ListParams, fields: [SparsePluginField]) async throws -> [T] {
        let request = api.requestBuilder.plugins().filterList(context: context, params: params, fields: fields)
        let response = try await api.perform(request: request)
        return try parseFilterPluginsResponse(response: response)
    }

    public func get(id: T.ID, fields: [SparsePluginField]) async throws -> T {
        let request = api.requestBuilder.plugins().filterRetrieve(context: context, plugin: id, fields: fields)
        let response = try await api.perform(request: request)
        return try parseFilterRetrievePluginResponse(response: response)
    }
}

