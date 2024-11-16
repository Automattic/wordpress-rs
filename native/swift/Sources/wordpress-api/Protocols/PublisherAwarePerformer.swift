import Foundation
import WordPressAPIInternal
import Combine

public protocol PublisherAwarePerformer: RequestPerformer {

//    TODO
//    func publisherWithCreate(params: CreateParamsType) -> AnyPublisher<CreateResponseType, Error>
//    func publisherWithUpdate(id: IdType, params: UpdateParamsType) -> AnyPublisher<UpdateResponseType, Error>
//    func publisherWithDelete(id: IdType) -> AnyPublisher<DeleteResponseType, Error>

    // Generated implementation
    func publisherWithEditContext(
        params: ListParamsType
    ) -> AnyPublisher<EditContextListResponseType, Error>

    func publisherWithViewContext(
        params: ListParamsType
    ) -> AnyPublisher<ViewContextListResponseType, Error>

    func publisherWithEmbedContext(
        params: ListParamsType
    ) -> AnyPublisher<EmbedContextListResponseType, Error>
}

extension PublisherAwarePerformer {
    func perform(request: WpNetworkRequest) -> Publishers.TryMap<URLSession.DataTaskPublisher, WpNetworkResponse> {
        URLSession.shared.dataTaskPublisher(for: request.asURLRequest())
            .tryMap { try WpNetworkResponse.from(data: $0.data, response: $0.response) }
    }

    public func publisherWithEditContext(
        params: ListParamsType
    ) -> AnyPublisher<EditContextListResponseType, Error> {
        perform(request: buildListWithEditRequest(params: params))
            .tryMap { try parseListWithEditResponse(response: $0) }
            .eraseToAnyPublisher()
    }

    public func publisherWithViewContext(
        params: ListParamsType
    ) -> AnyPublisher<ViewContextListResponseType, Error> {
        perform(request: buildListWithViewRequest(params: params))
            .tryMap { try parseListWithViewResponse(response: $0) }
            .eraseToAnyPublisher()
    }

    public func publisherWithEmbedContext(
        params: ListParamsType
    ) -> AnyPublisher<EmbedContextListResponseType, Error> {
        perform(request: buildListWithEmbedRequest(params: params))
            .tryMap { try parseListWithEmbedResponse(response: $0) }
            .eraseToAnyPublisher()
    }
}
