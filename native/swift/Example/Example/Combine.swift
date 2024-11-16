import Foundation
import Combine
import SwiftUI
import WordPressAPI

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

        do {
            let currentStream = try self.streamProvider()

            currentStream.publisher
                .sink { completion in
                    switch completion {
                    case .finished:
                        self.isLoading = false

                    case .failure(let error):
                        self.error = MyError(underlyingError: error)
                        self.shouldPresentAlert = true
                    }
                } receiveValue: { allItems in
                    for item in allItems {
                        self.listItems[item.id] = item
                    }
                }
                .store(in: &cancellables)

        } catch {
            self.error = MyError(underlyingError: error)
            self.shouldPresentAlert = true
        }
    }
}

struct ListViewDataStream {
    let publisher: AnyPublisher<[ListViewData], Error>
}
