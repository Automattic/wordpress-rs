import Foundation

#if os(Linux)
import FoundationNetworking
#endif

public protocol RequestPerformer: Sendable {
    associatedtype IdType

    associatedtype SingleEditType
    associatedtype SingleEmbedType
    associatedtype SingleViewType

    associatedtype ListParamsType
    associatedtype CreateParamsType
    associatedtype UpdateParamsType

    associatedtype CreateResponseType
    associatedtype UpdateResponseType
    associatedtype DeleteResponseType

    associatedtype ListWithEditContextResponseType: PaginatableResponse
    associatedtype ListWithEmbedContextResponseType: PaginatableResponse
    associatedtype ListWithViewContextResponseType: PaginatableResponse

    var urlSession: URLSession { get }

    // MARK: - Request Builders
    func buildCreateRequest(params: CreateParamsType) -> WpNetworkRequest
    func buildUpdateRequest(id: IdType, params: UpdateParamsType) -> WpNetworkRequest

    func buildListWithEditRequest(params: ListParamsType) -> WpNetworkRequest
    func buildListWithEmbedRequest(params: ListParamsType) -> WpNetworkRequest
    func buildListWithViewRequest(params: ListParamsType) -> WpNetworkRequest

    // MARK: - Response Parsers
    func parseCreateResponse(response: WpNetworkResponse) throws -> CreateResponseType
    func parseUpdateResponse(response: WpNetworkResponse) throws -> UpdateResponseType
    func parseDeleteResponse(response: WpNetworkResponse) throws -> DeleteResponseType
    func parseListWithEditResponse(response: WpNetworkResponse) throws -> ListWithEditContextResponseType
    func parseListWithEmbedResponse(response: WpNetworkResponse) throws -> ListWithEmbedContextResponseType
    func parseListWithViewResponse(response: WpNetworkResponse) throws -> ListWithViewContextResponseType

    // MARK: – Known-to-exist List Methods
    func listWithEditContext(
        params: ListWithEditContextResponseType.ParamsType
    ) async throws -> ListWithEditContextResponseType

    func listWithEmbedContext(
        params: ListWithEmbedContextResponseType.ParamsType
    ) async throws -> ListWithEmbedContextResponseType

    func listWithViewContext(
        params: ListWithViewContextResponseType.ParamsType
    ) async throws -> ListWithViewContextResponseType
}

public protocol NoDeletionParams {
    associatedtype IdType
    func buildDeleteRequest(id: IdType) -> WpNetworkRequest
}

public protocol HasDeletionParams {
    associatedtype IdType
    associatedtype DeleteParamsType
    func buildDeleteRequest(id: IdType, params: DeleteParamsType) -> WpNetworkRequest
}
