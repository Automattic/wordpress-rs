import Foundation
import SwiftUI
import WordPressAPI
import WordPressAPICombine
import Combine

@MainActor
protocol ListViewModel {

    /// Guarantee only one object with each ID, but allow updating the object when new data comes in
    var listItems: [String: ListViewData] { get }

    var shouldPresentAlert: Bool { get set }

    var error: MyError? { get set }

    func task() async
}

@Observable class SequenceListViewModel: ListViewModel {
    var listItems: [String: ListViewData] = [String: ListViewData](minimumCapacity: 250)

    typealias SequenceProvider = () -> ListViewSequence

    private let sequenceProvider: SequenceProvider

    init(sequenceProvider: @escaping SequenceProvider) {
        self.sequenceProvider = sequenceProvider
    }

    var shouldPresentAlert: Bool = false

    var error: MyError?

    var sequence: ListViewSequence?

    func task() async {
        do {
            for try await page in self.sequenceProvider() {
                for item in page {
                    self.listItems[item.id] = item
                }
            }
        } catch {
            self.error = .init(underlyingError: error)
            self.shouldPresentAlert = true
        }
    }

    func reset() {

    }
}

@Observable class TaskListViewModel: ListViewModel {

    typealias FetchDataTask = () async throws -> [ListViewData]

    var listItems: [String: ListViewData] = [:]
    private var dataCallback: FetchDataTask
    var isLoading: Bool = false

    var error: MyError?
    var shouldPresentAlert = false

    init(dataCallback: @escaping FetchDataTask) {
        self.dataCallback = dataCallback
    }

    func task() async {
        self.isLoading = true
        self.shouldPresentAlert = false

        do {
            for item in try await dataCallback() {
                listItems[item.id] = item
            }
        } catch {
            self.error = MyError(underlyingError: error)
            self.shouldPresentAlert = true
        }

        self.isLoading = false
    }
}

@Observable class CombineListViewModel: ListViewModel {

    public typealias StreamProvider = () throws -> ListViewDataStream

    var listItems: [String: ListViewData] = [:]
    var isLoading: Bool = false

    var shouldPresentAlert: Bool = false

    var error: MyError?
    var cancellables: Set<AnyCancellable> = []

    private let streamProvider: StreamProvider

    init(streamProvider: @escaping StreamProvider) {
        self.streamProvider = streamProvider
    }

    func task() async {
        self.error = nil

        guard var currentStream = try? self.streamProvider() else { return }

        currentStream.getPublisher()
            .sink { completion in
                switch completion {
                case .finished:
                    self.isLoading = false

                case .failure(let error):
                    self.error = MyError(underlyingError: error)
                    self.shouldPresentAlert = true
                }
            } receiveValue: { newValue in
                withAnimation {
                    for item in newValue {
                        self.listItems[item.id] = item
                    }
                }
            }
        .store(in: &cancellables)

        try? await currentStream.fetch()
    }
}

struct MyError: LocalizedError {
    var underlyingError: Error

    var localizedDescription: String {
        underlyingError.localizedDescription
    }

    var errorDescription: String? {
        "Unable to fetch data"
    }

    var failureReason: String? {
        underlyingError.localizedDescription
    }
}

struct ListViewDataStream {
    typealias ValueType = [ListViewData]

    let publisher: AnyPublisher<[ListViewData], Error>
    var underlyingStream: WordPressAPICombine.Fetchable

    mutating func fetch() async throws {
        try await self.underlyingStream.fetch()
    }

    func getPublisher() -> AnyPublisher<[ListViewData], any Error> {
        publisher
    }
}

struct ListViewSequence: AsyncSequence {
    typealias Element = [ListViewData]

    private let underlyingSequence: any AsyncSequence

    init(underlyingSequence: any AsyncSequence) {
        self.underlyingSequence = underlyingSequence
    }

    struct ListViewIterator: AsyncIteratorProtocol {
        var underlyingSequence: any AsyncIteratorProtocol

        mutating func next() async throws -> Element? {
            guard let nextElement = try await underlyingSequence.next() else {
                return nil
            }

            guard let listViewData = nextElement as? [any ListViewDataConvertable] else {
                debugPrint("Unable to convert data to `ListViewDataConvertable`")
                return nil
            }

            return listViewData.asListViewData()
        }
    }

    func makeAsyncIterator() -> ListViewIterator {
        ListViewIterator(underlyingSequence: underlyingSequence.makeAsyncIterator())
    }
}
