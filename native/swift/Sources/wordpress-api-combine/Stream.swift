import Combine
import WordPressAPI

public extension PaginationAwareExecutor {
    func streamWithEditContext(
        params: Self.EditContextResponseType.ParamsType
    ) -> Stream<Self, Self.EditContextResponseType> {
        Stream(executor: self, params: params, sequence: self.sequenceWithEditContext(params: params) )
    }

    func streamWithViewContext(
        params: Self.ViewContextResponseType.ParamsType
    ) -> Stream<Self, Self.ViewContextResponseType> {
        Stream(executor: self, params: params, sequence: self.sequenceWithViewContext(params: params) )
    }

    func streamWithEmbedContext(
        params: Self.EmbedContextResponseType.ParamsType
    ) -> Stream<Self, Self.EmbedContextResponseType> {
        Stream(executor: self, params: params, sequence: self.sequenceWithEmbedContext(params: params) )
    }
}

public protocol Fetchable {
    mutating func fetch() async throws
}

public struct Stream<Executor: PaginationAwareExecutor, ResponseType: PaginatableResponse>: Fetchable {
    private let publisher = CurrentValueSubject<[ResponseType.DataType], Error>([])
    private let executor: Executor
    private let params: ResponseType.ParamsType
    private let sequence: PaginationSequence<ResponseType>

    private var accumulatedObjects: [ResponseType.DataType] = []

    public init(executor: Executor, params: ResponseType.ParamsType, sequence: PaginationSequence<ResponseType>) {
        self.executor = executor
        self.params = params
        self.sequence = sequence
    }

    public func getPublisher() -> AnyPublisher<[ResponseType.DataType], Error> {
        publisher.eraseToAnyPublisher()
    }

    public mutating func fetch() async throws {
        self.publisher.send(self.accumulatedObjects)

        do {
            for try await objects in sequence {
                accumulatedObjects.append(contentsOf: objects)
                publisher.send(accumulatedObjects)
            }
        } catch {
            publisher.send(completion: .failure(error))
        }
    }
}
