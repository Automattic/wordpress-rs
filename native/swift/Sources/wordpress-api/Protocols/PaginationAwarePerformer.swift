import Foundation

public protocol PaginatableResponse<ParamsType, DataType>: Sendable {
    associatedtype ParamsType
    associatedtype DataType

    var nextPageParams: ParamsType? { get }
    var prevPageParams: ParamsType? { get }

    var data: [DataType] { get }

    init(data: [DataType], headerMap: WpNetworkHeaderMap, nextPageParams: ParamsType?, prevPageParams: ParamsType?)
}

public protocol PaginationAwarePerformer: RequestPerformer {
    func paginatedWithEditContext(
        params: ListWithEditContextResponseType.ParamsType
    ) async throws -> [ListWithEditContextResponseType.DataType]

    func paginatedWithEmbedContext(
        params: ListWithEmbedContextResponseType.ParamsType
    ) async throws -> [ListWithEmbedContextResponseType.DataType]

    func paginatedWithViewContext(
        params: ListWithViewContextResponseType.ParamsType
    ) async throws -> [ListWithViewContextResponseType.DataType]
}

extension PaginationAwarePerformer {

    /// Fetches all objects from all pages
    ///
    /// This method waits until all objects have been downloaded then returns the results. This can have
    /// unexpected memory and time implications.
    public func paginatedWithEditContext(
        params: ListWithEditContextResponseType.ParamsType
    ) async throws -> [ListWithEditContextResponseType.DataType] {
        var allObjects: [ListWithEditContextResponseType.DataType] = []
        var mutableParams: ListWithEditContextResponseType.ParamsType = params

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
        params: ListWithViewContextResponseType.ParamsType
    ) async throws -> [ListWithViewContextResponseType.DataType] {
        var allObjects: [ListWithViewContextResponseType.DataType] = []
        var mutableParams: ListWithViewContextResponseType.ParamsType = params

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
        params: ListWithEmbedContextResponseType.ParamsType
    ) async throws -> [ListWithEmbedContextResponseType.DataType] {
        var allObjects: [ListWithEmbedContextResponseType.DataType] = []
        var mutableParams: ListWithEmbedContextResponseType.ParamsType = params

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
