import Foundation
import SwiftUI
import wordpress_api

@Observable class PostListViewModel {

    static let shared = PostListViewModel()

    var posts: PostCollection
    var fetchPostsTask: Task<Void, Never>?
    var error: MyError?
    var shouldPresentAlert = false

    private let api = WordPressAPI(
        urlSession: .shared,
        baseUrl: URL(string: "https://sweetly-unadulterated.jurassic.ninja")!,
        authenticationStategy: .init(username: "demo", password: "qD6z ty5l oLnL gXVe 0UED qBUB")
    )

    init(posts: PostCollection = PostCollection()) {
        self.posts = posts
    }

    func startFetchingPosts() {
        self.error = nil
        self.shouldPresentAlert = false

        self.fetchPostsTask = Task { @MainActor in
            do {
                self.posts = try await api.listPostsRust()
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
