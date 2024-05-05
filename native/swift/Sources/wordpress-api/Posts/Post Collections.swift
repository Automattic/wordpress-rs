import Foundation
import WordPressAPIInternal

extension PostObject: Identifiable {
    // swiftlint:disable identifier_name
    var ID: any Hashable {
        self.id
    }
    // swiftlint:enable identifier_name
}

public typealias PostCollection = [PostObject]

public struct PostObjectSequence: AsyncSequence, AsyncIteratorProtocol {
    public typealias Element = PostObject

    private let api: WordPressAPI

    private var posts: [PostObject] = []
    private var nextPage: WpNetworkRequest?

    enum Errors: Error {
        case unableToFetchPosts
    }

    init(api: WordPressAPI, initialParams: PostListParams) {
        self.api = api
        self.nextPage = api.helper.postListRequest(params: initialParams)
    }

    mutating public func next() async throws -> PostObject? {
        if posts.isEmpty {
            guard let nextPage = self.nextPage else {
                return nil
            }

            try await fetchMorePosts(with: nextPage)
        }

        return posts.removeFirst()
    }

    private mutating func fetchMorePosts(with request: WpNetworkRequest) async throws {
        let rawResponse = try await api.perform(request: request)
        let parsedResponse = try parsePostListResponse(response: rawResponse)

        if let postList = parsedResponse.postList {
            self.posts.append(contentsOf: postList)
        } else {
            throw Errors.unableToFetchPosts
        }

        if let nextPageUri = parsedResponse.nextPage {
            self.nextPage = self.api.helper.rawRequest(url: nextPageUri)
        } else {
            self.nextPage = nil
        }
    }

    public func makeAsyncIterator() -> PostObjectSequence {
        self
    }
}
