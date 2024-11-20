import Foundation
import Combine

public protocol PublisherAwarePerformer: RequestPerformer {

    func publisherWithCreate(params: CreateParamsType) -> AnyPublisher<CreateResponseType, Error>
    func publisherWithUpdate(id: IdType, params: UpdateParamsType) -> AnyPublisher<UpdateResponseType, Error>

    // Generated implementation
    func publisherWithEditContext(
        params: ListWithEditContextResponseType.ParamsType
    ) -> AnyPublisher<[ListWithEditContextResponseType.DataType], Error>

    func publisherWithEmbedContext(
        params: ListWithEmbedContextResponseType.ParamsType
    ) -> AnyPublisher<[ListWithEmbedContextResponseType.DataType], Error>

    func publisherWithViewContext(
        params: ListWithViewContextResponseType.ParamsType
    ) -> AnyPublisher<[ListWithViewContextResponseType.DataType], Error>
}

extension PublisherAwarePerformer {

    public func publisherWithCreate(
        params: CreateParamsType
    ) -> AnyPublisher<CreateResponseType, any Error> {
        perform(request: buildCreateRequest(params: params)).tryMap {
            try parseCreateResponse(response: $0)
        }.eraseToAnyPublisher()
    }

    public func publisherWithUpdate(
        id: IdType,
        params: UpdateParamsType
    ) -> AnyPublisher<UpdateResponseType, any Error> {
        perform(request: buildUpdateRequest(id: id, params: params)).tryMap {
            try parseUpdateResponse(response: $0)
        }.eraseToAnyPublisher()
    }

    // swiftlint:disable force_cast
    public func publisherWithEditContext(
        params: ListWithEditContextResponseType.ParamsType
    ) -> AnyPublisher<[ListWithEditContextResponseType.DataType], Error> {
        recursivePublisher(
            params: params,
            requestTransformer: { buildListWithEditRequest(params: $0 as! Self.ListParamsType) },
            responseTransformer: { try parseListWithEditResponse(response: $0) }
        )
    }

    public func publisherWithViewContext(
        params: ListWithViewContextResponseType.ParamsType
    ) -> AnyPublisher<[ListWithViewContextResponseType.DataType], Error> {
        recursivePublisher(
            params: params,
            requestTransformer: { buildListWithViewRequest(params: $0 as! Self.ListParamsType) },
            responseTransformer: { try parseListWithViewResponse(response: $0) }
        )
    }

    public func publisherWithEmbedContext(
        params: ListWithEmbedContextResponseType.ParamsType
    ) -> AnyPublisher<[ListWithEmbedContextResponseType.DataType], Error> {
        recursivePublisher(
            params: params,
            requestTransformer: { buildListWithEmbedRequest(params: $0 as! Self.ListParamsType) },
            responseTransformer: { try parseListWithEmbedResponse(response: $0) }
        )
    }
    // swiftlint:enable force_cast

    public func recursivePublisher<T: PaginatableResponse>(
        params: T.ParamsType,
        requestTransformer: @escaping (T.ParamsType) -> WpNetworkRequest,
        responseTransformer: @escaping (WpNetworkResponse) throws -> T
    ) -> AnyPublisher<[T.DataType], Error> {
        let paramsPublisher = CurrentValueSubject<T.ParamsType, Error>(params)

        return paramsPublisher                                    // Following Combine chains can be tricky so:
            .flatMap { perform(request: requestTransformer($0)) } // 1. `dataTaskPublisher` replaces `paramsPublisher`
            .tryMap { try responseTransformer($0) }               // 2. Map the raw response back to the expected type
            .handleEvents(receiveOutput: { output in              // 3. When a new event comes in...
                if let nextPageParams = output.nextPageParams {   //    If there's another page...
                    paramsPublisher.send(nextPageParams)          //       Kick off the paramsPublisher again
                } else {                                          //    Otherwise we're done
                    paramsPublisher.send(completion: .finished)   //       And paramsPublisher is complete
                }
            })
            .map { $0.data }                                      // 4. Drop the parsed response wrapper
            .eraseToAnyPublisher()
    }

    private func perform(
        request: WpNetworkRequest
    ) -> Publishers.TryMap<URLSession.DataTaskPublisher, WpNetworkResponse> {
        self.urlSession
            .dataTaskPublisher(for: request.asURLRequest())
            .tryMap { try WpNetworkResponse.from(data: $0.data, response: $0.response) }
    }
}

extension PublisherAwarePerformer where Self: NoDeletionParams {
    public func publisherWithDelete(
        id: IdType
    ) -> AnyPublisher<DeleteResponseType, any Error> {
        perform(request: buildDeleteRequest(id: id)).tryMap {
            try parseDeleteResponse(response: $0)
        }.eraseToAnyPublisher()
    }
}

extension PublisherAwarePerformer where Self: HasDeletionParams {
    public func publisherWithDelete(
        id: IdType,
        params: DeleteParamsType
    ) -> AnyPublisher<DeleteResponseType, any Error> {
        perform(request: buildDeleteRequest(id: id, params: params)).tryMap {
            try parseDeleteResponse(response: $0)
        }.eraseToAnyPublisher()
    }
}
