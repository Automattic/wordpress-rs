import Foundation

public protocol CallbackAwarePerformer: RequestPerformer {

    func create(
        params: CreateParamsType,
        queue: DispatchQueue,
        callback: @escaping @Sendable (Result<CreateResponseType, Error>) -> Void
    )

    func update(
        id: IdType,
        params: UpdateParamsType,
        queue: DispatchQueue,
        callback: @escaping @Sendable (Result<UpdateResponseType, Error>) -> Void
    )

    // Generated implementation
    func listWithEditContext(
        params: ListParamsType,
        queue: DispatchQueue,
        callback: @escaping @Sendable (Result<EditContextListResponseType, Error>) -> Void
    )

    func listWithViewContext(
        params: ListParamsType,
        queue: DispatchQueue,
        callback: @escaping @Sendable (Result<ViewContextListResponseType, Error>) -> Void
    )

    func listWithEmbedContext(
        params: ListParamsType,
        queue: DispatchQueue,
        callback: @escaping @Sendable (Result<EmbedContextListResponseType, Error>) -> Void
    )
}

public protocol CallbackAwareRequestPerformer: RequestPerformer, NoDeletionParams {
    func delete(
        id: IdType,
        queue: DispatchQueue,
        callback: @escaping @Sendable (Result<DeleteResponseType, Error>) -> Void
    )
}

extension CallbackAwarePerformer {

    public func create(
        params: CreateParamsType,
        queue: DispatchQueue,
        callback: @escaping @Sendable (Result<CreateResponseType, Error>) -> Void
    ) {
        perform(request: buildCreateRequest(params: params), on: queue, responseConverter: {
            try parseCreateResponse(response: $0)
        }, completion: callback)
    }

    public func update(
        id: IdType,
        params: UpdateParamsType,
        queue: DispatchQueue,
        callback: @escaping @Sendable (Result<UpdateResponseType, any Error>) -> Void
    ) {
        perform(request: buildUpdateRequest(id: id, params: params), on: queue, responseConverter: {
            try parseUpdateResponse(response: $0)
        }, completion: callback)
    }

    public func listWithEditContext(
        params: ListParamsType,
        queue: DispatchQueue,
        callback: @escaping @Sendable (Result<EditContextListResponseType, Error>) -> Void
    ) {
        perform(request: buildListWithEditRequest(params: params), on: queue, responseConverter: {
            try parseListWithEditResponse(response: $0)
        }, completion: callback)
    }

    public func listWithEmbedContext(
        params: ListParamsType,
        queue: DispatchQueue,
        callback: @escaping @Sendable (Result<EmbedContextListResponseType, Error>) -> Void
    ) {
        perform(request: buildListWithEmbedRequest(params: params), on: queue, responseConverter: {
            try parseListWithEmbedResponse(response: $0)
        }, completion: callback)
    }

    public func listWithViewContext(
        params: ListParamsType,
        queue: DispatchQueue,
        callback: @escaping @Sendable (Result<ViewContextListResponseType, Error>) -> Void
    ) {
        perform(request: buildListWithViewRequest(params: params), on: queue, responseConverter: {
            try parseListWithViewResponse(response: $0)
        }, completion: callback)
    }

    private func perform<ResponseType>(
        request: WpNetworkRequest,
        on queue: DispatchQueue = .global(qos: .background),
        responseConverter: @escaping @Sendable (WpNetworkResponse) throws -> ResponseType,
        completion: @escaping @Sendable (Result<ResponseType, Error>) -> Void
    ) {
        self.urlSession.dataTask(with: request.asURLRequest()) { data, response, error in
            queue.async {
                if let error {
                    completion(.failure(error))
                    return
                }

                guard let httpResponse = response as? HTTPURLResponse, let data else {
                    completion(.failure(WordPressAPI.Errors.unableToParseResponse))
                    return
                }

                do {
                    let rawResponse = try WpNetworkResponse.from(data: data, response: httpResponse)
                    let parsedResponse = try responseConverter(rawResponse)
                    completion(.success(parsedResponse))
                } catch {
                    completion(.failure(error))
                }
            }
        }
    }
}

extension CallbackAwarePerformer where Self: NoDeletionParams {
    public func delete(
        id: IdType,
        queue: DispatchQueue,
        callback: @escaping @Sendable (Result<DeleteResponseType, Error>) -> Void
    ) {
        perform(request: buildDeleteRequest(id: id), on: queue, responseConverter: {
            try parseDeleteResponse(response: $0)
        }, completion: callback)
    }

}
