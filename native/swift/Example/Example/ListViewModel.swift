import Foundation
import WordPressApiCache
import SwiftUI
import WordPressAPI

@MainActor
protocol ListViewModel {

    /// Guarantee only one object with each ID, but allow updating the object when new data comes in
    var listItems: [String: ListViewData] { get }

    var shouldPresentAlert: Bool { get set }

    var error: MyError? { get set }

    func task() async

    func triggerUpdate() async
}

@Observable class SequenceListViewModel: ListViewModel {
    var listItems: [String: ListViewData] = [String: ListViewData](minimumCapacity: 250)

    typealias SequenceProvider = @Sendable () async throws -> ListViewSequence

    private let sequenceProvider: SequenceProvider

    init(sequenceProvider: @escaping SequenceProvider) {
        self.sequenceProvider = sequenceProvider
    }

    var shouldPresentAlert: Bool = false

    var error: MyError?

    var sequence: ListViewSequence?

    func task() async {
        do {
            for try await page in try await self.sequenceProvider() {
                for item in page {
                    self.listItems[item.id] = item
                }
            }
        } catch {
            self.error = MyError(underlyingError: error)
            self.shouldPresentAlert = true
        }
    }

    func triggerUpdate() async {
        // Do nothing
    }

    func reset() {

    }
}

@Observable class TaskListViewModel: ListViewModel {

    typealias FetchDataTask = @Sendable () async throws -> [ListViewData]

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

    func triggerUpdate() async {
        // Do nothing
    }
}

@Observable class CollectionListViewModel: ListViewModel {

    typealias CachedResultProvider = @Sendable () async throws -> [ListViewData]
    typealias FetchedResultsProvider = @Sendable (() async -> Void) async throws -> [ListViewData]

    var listItems: [String : ListViewData] = [:]

    var shouldPresentAlert: Bool = false

    var error: MyError? = nil

    private let cachePromise: CachedResultProvider
    private let fetchPromise: FetchedResultsProvider

    init(cachedResults: @escaping CachedResultProvider, fetchedResults: @escaping FetchedResultsProvider) {
        self.cachePromise = cachedResults
        self.fetchPromise = fetchedResults
    }

    func task() async {
        do {
            for item in try await cachePromise() {
                self.listItems[item.id] = item
            }

            for item in try await fetchPromise(self.triggerUpdate) {
                self.listItems[item.id] = item
            }
        } catch {
            self.error = MyError(underlyingError: error)
            self.shouldPresentAlert = true
        }
    }

    func triggerUpdate() async {

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

struct ListViewSequence: AsyncSequence {
    typealias Element = [ListViewData]

    private let underlyingSequence: any (AsyncSequence & Sendable)

    init(underlyingSequence: any (AsyncSequence & Sendable)) {
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
