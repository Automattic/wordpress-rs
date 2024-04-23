import Foundation
import SwiftUI
import wordpress_api

extension SparsePost.ViewContext: Identifiable {}

@Observable class PostListViewModel {

    var posts: [SparsePost.ViewContext]
    var fetchPostsTask: Task<Void, Never>?
    var error: MyError?
    var shouldPresentAlert = false

    let loginManager: LoginManager

    // swiftlint:disable force_try
    var api: WordPressAPI {
        WordPressAPI(
            urlSession: .shared,
            baseUrl: URL(string: loginManager.getDefaultSiteUrl()!)!,
            authenticationStategy: try! loginManager.getLoginCredentials()!
        )
    }
    // swiftlint:enable force_try

    init(loginManager: LoginManager, posts: [SparsePost.ViewContext] = []) {
        self.loginManager = loginManager
        self.posts = posts
    }

    func startFetchingPosts() {
        self.error = nil
        self.shouldPresentAlert = false

        self.fetchPostsTask = Task { @MainActor in
            do {
                for try await post in api.listPosts() {
                    posts.append(post)
                }
            } catch let error {
                shouldPresentAlert = true
                self.error = MyError(underlyingError: error)
                debugPrint(error.localizedDescription)
            }
        }
    }

    func stopFetchingPost() {
        self.fetchPostsTask?.cancel()
    }
}

struct MyError: LocalizedError {
    var underlyingError: Error

    var localizedDescription: String {
        underlyingError.localizedDescription
    }

    var errorDescription: String? {
        "Unable to fetch posts"
    }

    var failureReason: String? {
        underlyingError.localizedDescription
    }
}
