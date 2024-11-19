import Foundation

protocol InternalRequestPerformer {
    associatedtype ExecutorType
    associatedtype RequestBuilderType

    var executor: ExecutorType { get }
    var builder: RequestBuilderType { get }
}

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

    associatedtype EditContextListResponseType: PaginatableResponse
    associatedtype EmbedContextListResponseType: PaginatableResponse
    associatedtype ViewContextListResponseType: PaginatableResponse

    var urlSession: URLSession { get }

    // MARK: - Request Builders
    func buildCreateRequest(params: CreateParamsType) -> WpNetworkRequest
    func buildUpdateRequest(id: IdType, params: UpdateParamsType) -> WpNetworkRequest
    func buildDeleteRequest(id: IdType) -> WpNetworkRequest

    func buildListWithEditRequest(params: ListParamsType) -> WpNetworkRequest
    func buildListWithEmbedRequest(params: ListParamsType) -> WpNetworkRequest
    func buildListWithViewRequest(params: ListParamsType) -> WpNetworkRequest

    // MARK: - Response Parsers
    func parseCreateResponse(response: WpNetworkResponse) throws -> CreateResponseType
    func parseUpdateResponse(response: WpNetworkResponse) throws -> UpdateResponseType
    func parseDeleteResponse(response: WpNetworkResponse) throws -> DeleteResponseType

    func parseListWithEditResponse(response: WpNetworkResponse) throws -> EditContextListResponseType
    func parseListWithEmbedResponse(response: WpNetworkResponse) throws -> EmbedContextListResponseType
    func parseListWithViewResponse(response: WpNetworkResponse) throws -> ViewContextListResponseType
}
