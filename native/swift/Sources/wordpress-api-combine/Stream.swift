import Combine
import WordPressAPI

public extension PaginationAwareExecutor {
    func paginatedEditStream(params: Self.EditContextResponseType.ParamsType) -> Stream<Self, Self.EditContextResponseType> {
        Stream(executor: self, params: params) { executor, params in
            executor.paginatedSequenceWithEditContext(params: params)
        }
    }

    func paginatedViewStream(params: Self.ViewContextResponseType.ParamsType) -> Stream<Self, Self.ViewContextResponseType> {
        Stream(executor: self, params: params) { executor, params in
            executor.paginatedSequenceWithViewContext(params: params)
        }
    }

    func paginatedEmbedStream(params: Self.EmbedContextResponseType.ParamsType) -> Stream<Self, Self.EmbedContextResponseType> {
        Stream(executor: self, params: params) { executor, params in
            executor.paginatedSequenceWithEmbedContext(params: params)
        }
    }
}

public class Stream<Executor: PaginationAwareExecutor, ResponseType: PaginatableResponse> {
    private let publisher: CurrentValueSubject<[ResponseType.DataType], Error>
    private let executor: Executor
    private let params: ResponseType.ParamsType

    var accumulatedObjects: [ResponseType.DataType] = []

    public typealias StreamProvider = (Executor, ResponseType.ParamsType) -> PaginationSequence<ResponseType>

    var streamProvider: StreamProvider

    public init(executor: Executor, params: ResponseType.ParamsType, streamProvider: @escaping StreamProvider) {
        self.publisher = CurrentValueSubject([])
        self.executor = executor
        self.params = params
        self.streamProvider = streamProvider
    }

    public func getPublisher() -> AnyPublisher<[ResponseType.DataType], Error> {
        publisher.eraseToAnyPublisher()
    }

    public func fetch() async throws {
        self.accumulatedObjects.append(contentsOf: [])
        self.publisher.send(self.accumulatedObjects)

        do {
            for try await objects in streamProvider(executor, params) {
                accumulatedObjects.append(contentsOf: objects)
                publisher.send(accumulatedObjects)
            }
        } catch {
            publisher.send(completion: .failure(error))
        }
    }
}
