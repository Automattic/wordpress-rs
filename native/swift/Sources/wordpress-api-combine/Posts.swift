import Combine
import WordPressAPI

public extension PostsRequestExecutor {
    func paginatedStream(params: PostListParams) -> PostsStream {
        PostsStream(executor: self, params: params)
    }
}

public protocol Stream<Element> {
    associatedtype Element

    func fetch() async throws
    func getPublisher() -> AnyPublisher<Element, Error>
}

public class PostsStream: Stream {
    private let publisher: CurrentValueSubject<[PostWithEditContext], Error>
    private let executor: PostsRequestExecutor
    private let params: PostListParams

    private var accumulatedObjects: [PostWithEditContext] = []

    public init(executor: PostsRequestExecutor, params: PostListParams) {
        self.publisher = CurrentValueSubject([])
        self.executor = executor
        self.params = params
    }

    public func getPublisher() -> AnyPublisher<[PostWithEditContext], any Error> {
        publisher.eraseToAnyPublisher()
    }

    public func fetch() async throws {
        self.accumulatedObjects = []
        self.publisher.send(self.accumulatedObjects)

        do {
            for try await posts in executor.paginatedSequenceWithEditContext(params: params) {
                accumulatedObjects.append(contentsOf: posts)
                publisher.send(accumulatedObjects)
            }
        } catch {
            publisher.send(completion: .failure(error))
        }
    }
}
