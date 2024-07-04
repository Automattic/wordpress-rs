import Foundation
import SwiftUI
import WordPressAPI

@Observable class ListViewModel {

    var listItems: [ListViewData] = []
    var fetchDataTask: Task<[ListViewData], Error>
    var isLoading: Bool = false

    var error: MyError?
    var shouldPresentAlert = false

    let loginManager: LoginManager

    init(loginManager: LoginManager, fetchDataTask: Task<[ListViewData], Error>) {
        self.loginManager = loginManager
        self.fetchDataTask = fetchDataTask
    }

    func startFetching() {
        self.error = nil
        self.shouldPresentAlert = false

       Task { @MainActor in
           self.isLoading = true

           do {
               self.listItems = try await self.fetchDataTask.value
           } catch {
               self.error = MyError(underlyingError: error)
               self.shouldPresentAlert = true
           }

           self.isLoading = false
        }
    }

    func stopFetching() {
        self.fetchDataTask.cancel()
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
