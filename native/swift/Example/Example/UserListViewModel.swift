import Foundation
import SwiftUI
import WordPressAPI

#if hasFeature(RetroactiveAttribute)
extension UserWithViewContext: @retroactive Identifiable {}
#else
extension UserWithViewContext: Identifiable {}
#endif

@Observable class UserListViewModel {

    var users: [UserWithViewContext]
    var fetchUsersTask: Task<Void, Never>?
    var error: MyError?
    var shouldPresentAlert = false

    let loginManager: LoginManager

    // swiftlint:disable force_try
    var api: WordPressAPI {
        try! WordPressAPI(
            urlSession: .shared,
            baseUrl: URL(string: loginManager.getDefaultSiteUrl()!)!,
            authenticationStategy: try! loginManager.getLoginCredentials()!
        )
    }
    // swiftlint:enable force_try

    init(loginManager: LoginManager, users: [UserWithViewContext] = []) {
        self.loginManager = loginManager
        self.users = users
    }

    func startFetching() {
        self.error = nil
        self.shouldPresentAlert = false

        self.fetchUsersTask = Task { @MainActor in
            do {
                users = try await api.users.listWithViewContext(params: .init())
            } catch let error {
                shouldPresentAlert = true
                self.error = MyError(underlyingError: error)
                debugPrint(error.localizedDescription)
            }
        }
    }

    func stopFetching() {
        self.fetchUsersTask?.cancel()
    }
}

struct MyError: LocalizedError {
    var underlyingError: Error

    var localizedDescription: String {
        underlyingError.localizedDescription
    }

    var errorDescription: String? {
        "Unable to fetch users"
    }

    var failureReason: String? {
        underlyingError.localizedDescription
    }
}
