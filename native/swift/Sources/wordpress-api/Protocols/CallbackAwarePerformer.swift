import Foundation

public protocol CallbackAwarePerformer: RequestPerformer {

    // Generated implementation
    func listWithEditContext(
        params: ListParamsType,
        callback: @escaping @Sendable (Result<EditContextListResponseType, Error>) -> Void,
        queue: DispatchQueue
    )

    func listWithViewContext(
        params: ListParamsType,
        callback: @escaping @Sendable (Result<ViewContextListResponseType, Error>) -> Void,
        queue: DispatchQueue
    )

    func listWithEmbedContext(
        params: ListParamsType,
        callback: @escaping @Sendable (Result<EmbedContextListResponseType, Error>) -> Void,
        queue: DispatchQueue
    )
}

extension CallbackAwarePerformer {

    public func listWithEditContext(
        params: ListParamsType,
        callback: @escaping @Sendable (Result<EditContextListResponseType, Error>) -> Void,
        queue: DispatchQueue
    ) {
        perform(request: buildListWithEditRequest(params: params), callback: { result in
            switch result {
            case .success(let response):
                do {
                    let response = try parseListWithEditResponse(response: response)
                    callback(.success(response))
                } catch {
                    callback(.failure(error))
                }
            case .failure(let error):
                callback(.failure(error))
            }
        }, on: queue)
    }

    public func listWithEmbedContext(
        params: ListParamsType,
        callback: @escaping @Sendable (Result<EmbedContextListResponseType, Error>) -> Void,
        queue: DispatchQueue
    ) {
        perform(request: buildListWithEmbedRequest(params: params), callback: { result in
            switch result {
            case .success(let response):
                do {
                    let response = try parseListWithEmbedResponse(response: response)
                    callback(.success(response))
                } catch {
                    callback(.failure(error))
                }
            case .failure(let error):
                callback(.failure(error))
            }
        }, on: queue)
    }

    public func listWithViewContext(
        params: ListParamsType,
        callback: @escaping @Sendable (Result<ViewContextListResponseType, Error>) -> Void,
        queue: DispatchQueue
    ) {
        perform(request: buildListWithViewRequest(params: params), callback: { result in
            switch result {
            case .success(let response):
                do {
                    let response = try parseListWithViewResponse(response: response)
                    callback(.success(response))
                } catch {
                    callback(.failure(error))
                }
            case .failure(let error):
                callback(.failure(error))
            }
        }, on: queue)
    }

    private func perform(
        request: WpNetworkRequest,
        callback: @escaping @Sendable (Result<WpNetworkResponse, Error>) -> Void,
        on queue: DispatchQueue = .global(qos: .background)
    ) {
        self.urlSession.dataTask(with: request.asURLRequest()) { data, response, error in
            queue.async {
                if let error {
                    callback(.failure(error))
                    return
                }

                guard let httpResponse = response as? HTTPURLResponse, let data else {
                    callback(.failure(WordPressAPI.Errors.unableToParseResponse))
                    return
                }

                do {
                    let response = try WpNetworkResponse.from(data: data, response: httpResponse)
                    callback(.success(response))
                } catch {
                    callback(.failure(error))
                }
            }
        }
    }

}
