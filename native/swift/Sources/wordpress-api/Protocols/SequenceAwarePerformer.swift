import Foundation

protocol SequenceAwarePerformer: RequestPerformer {
    // MARK: - Pagination
    func sequenceWithEditContext(
        params: ListWithEditContextResponseType.ParamsType
    ) -> PaginationSequence<ListWithEditContextResponseType>

    func sequenceWithViewContext(
        params: ListWithViewContextResponseType.ParamsType
    ) -> PaginationSequence<ListWithViewContextResponseType>

    func sequenceWithEmbedContext(
        params: ListWithEmbedContextResponseType.ParamsType
    ) -> PaginationSequence<ListWithEmbedContextResponseType>
}

extension SequenceAwarePerformer {

    public func sequenceWithEditContext(
        params: ListWithEditContextResponseType.ParamsType
    ) -> PaginationSequence<ListWithEditContextResponseType> {
        PaginationSequence(params: params) { params in
            try await self.listWithEditContext(params: params)
        }
    }

    public func sequenceWithEmbedContext(
        params: ListWithEmbedContextResponseType.ParamsType
    ) -> PaginationSequence<ListWithEmbedContextResponseType> {
        PaginationSequence(params: params) { params in
            try await self.listWithEmbedContext(params: params)
        }
    }

    public func sequenceWithViewContext(
        params: ListWithViewContextResponseType.ParamsType
    ) -> PaginationSequence<ListWithViewContextResponseType> {
        PaginationSequence(params: params) { params in
            try await self.listWithViewContext(params: params)
        }
    }
}

public struct PaginationSequence<ResponseType: PaginatableResponse>: AsyncSequence {
    public typealias Transformer = (ResponseType.ParamsType) async throws -> ResponseType

    private let params: ResponseType.ParamsType
    private let transform: Transformer

    init(params: ResponseType.ParamsType, transform: @escaping Transformer) {
        self.params = params
        self.transform = transform
    }

    public struct AsyncIterator: AsyncIteratorProtocol {
        private var nextPageParams: ResponseType.ParamsType?
        private let transform: Transformer

        init(params: ResponseType.ParamsType, transform: @escaping Transformer) {
            self.nextPageParams = params
            self.transform = transform
        }

        public mutating func next() async throws -> [ResponseType.DataType]? {
            guard let nextPageParams else {
                return nil
            }

            let response = try await self.transform(nextPageParams)
            self.nextPageParams = response.nextPageParams
            return response.data
        }
    }

    public func makeAsyncIterator() -> AsyncIterator {
        AsyncIterator(params: params, transform: self.transform)
    }
}
