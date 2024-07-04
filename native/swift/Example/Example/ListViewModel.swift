import Foundation
import SwiftUI
import WordPressAPI

@Observable class ListViewModel {

    typealias FetchDataTask = () async throws -> [ListViewData]

    var listItems: [ListViewData] = []
    private var dataCallback: FetchDataTask
    private var dataTask: Task<Void, any Error>?
    var isLoading: Bool = false

    var error: MyError?
    var shouldPresentAlert = false

    let loginManager: LoginManager

    init(loginManager: LoginManager, dataCallback: @escaping FetchDataTask) {
        self.loginManager = loginManager
        self.dataCallback = dataCallback
    }

    func startFetching() {
        self.error = nil
        self.shouldPresentAlert = false

        self.dataTask = Task { @MainActor in
            self.isLoading = true
            self.shouldPresentAlert = false

            do {
                self.listItems = try await dataCallback()
            } catch {
                self.error = MyError(underlyingError: error)
                self.shouldPresentAlert = true
            }

            self.isLoading = false
        }
    }

    func stopFetching() {
        self.dataTask?.cancel()
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
