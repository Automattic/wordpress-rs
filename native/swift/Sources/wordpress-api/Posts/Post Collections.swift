import Foundation
import wordpress_api_wrapper

extension PostObject: Identifiable {
    var ID: any Hashable {
        self.id
    }
}

public typealias PostCollection = [PostObject]

extension WPEditContextPost: Identifiable {
    var ID: any Hashable {
        self.id
    }
}

public struct PostObjectSequence: AsyncSequence, AsyncIteratorProtocol {
    public typealias Element = PostObject

    private let api: WordPressAPI

    private var posts: [PostObject] = []
    private var nextPage: WpNetworkRequest?

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
            abort() // TODO: Not sure if this should be an error
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
