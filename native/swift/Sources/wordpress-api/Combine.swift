import Foundation
import WordPressAPIInternal
import Combine

public protocol PublisherAwareExecutor: PaginationAwareExecutor {

    var requestBuilder: PublisherRequestBuilder<Self> { get }
    var parser: ResponseParser<Self> { get }

    // Generated implementation
    func publisherWithEditContext(
        params: EditContextResponseType.ParamsType
    ) -> AnyPublisher<EditContextResponseType, Error>

    func publisherWithViewContext(
        params: ViewContextResponseType.ParamsType
    ) -> AnyPublisher<ViewContextResponseType, Error>

    func publisherWithEmbedContext(
        params: EmbedContextResponseType.ParamsType
    ) -> AnyPublisher<EmbedContextResponseType, Error>
}

public struct ResponseParser<T> where T: PublisherAwareExecutor {
    let editContextParser: (WpNetworkResponse) throws -> T.EditContextResponseType
    let viewContextParser: (WpNetworkResponse) throws -> T.ViewContextResponseType
    let embedContextParser: (WpNetworkResponse) throws -> T.EmbedContextResponseType
}

public struct PublisherRequestBuilder<T> where T: PublisherAwareExecutor {
    let editContextBuilder: (T.EditContextResponseType.ParamsType) -> WpNetworkRequest
    let viewContextBuilder: (T.ViewContextResponseType.ParamsType) -> WpNetworkRequest
    let embedContextBuilder: (T.EmbedContextResponseType.ParamsType) -> WpNetworkRequest
}

extension PublisherAwareExecutor {
    func perform(request: WpNetworkRequest) -> Publishers.TryMap<URLSession.DataTaskPublisher, WpNetworkResponse> {
        URLSession.shared.dataTaskPublisher(for: request.asURLRequest())
            .tryMap { try WpNetworkResponse.from(data: $0.data, response: $0.response) }
    }

    public func publisherWithEditContext(
        params: EditContextResponseType.ParamsType
    ) -> AnyPublisher<EditContextResponseType, Error> {
        perform(request: self.requestBuilder.editContextBuilder(params))
            .tryMap { try parser.editContextParser($0) }
            .eraseToAnyPublisher()
    }

    public func publisherWithViewContext(
        params: ViewContextResponseType.ParamsType
    ) -> AnyPublisher<ViewContextResponseType, Error> {
        perform(request: self.requestBuilder.viewContextBuilder(params))
            .tryMap { try parser.viewContextParser($0) }
            .eraseToAnyPublisher()
    }

    public func publisherWithEmbedContext(
        params: EmbedContextResponseType.ParamsType
    ) -> AnyPublisher<EmbedContextResponseType, Error> {
        perform(request: self.requestBuilder.embedContextBuilder(params))
            .tryMap { try parser.embedContextParser($0) }
            .eraseToAnyPublisher()
    }
}

extension PostsRequestExecutor: PublisherAwareExecutor {
    public var requestBuilder: PublisherRequestBuilder<WordPressAPIInternal.PostsRequestExecutor> {
        PublisherRequestBuilder { params in
            PostsRequestBuilder(noPointer: .init()).listWithEditContext(params: params)
        } viewContextBuilder: { params in
            PostsRequestBuilder(noPointer: .init()).listWithViewContext(params: params)
        } embedContextBuilder: { params in
            PostsRequestBuilder(noPointer: .init()).listWithEmbedContext(params: params)
        }
    }

    public var parser: ResponseParser<WordPressAPIInternal.PostsRequestExecutor> {
        ResponseParser(
            editContextParser: parseAsPostsRequestListWithEditContextResponse,
            viewContextParser: parseAsPostsRequestListWithViewContextResponse,
            embedContextParser: parseAsPostsRequestListWithEmbedContextResponse
        )
    }
}
