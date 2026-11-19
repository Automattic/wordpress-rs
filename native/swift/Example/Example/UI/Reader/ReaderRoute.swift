import SwiftUI
import WordPressAPI

/// A Reader screen to open.
enum ReaderRoute: Hashable, Sendable {
    case stream(ReaderStream)
    /// A post, by its key in the cache. `identity` is what to fetch it with when the cache doesn't have it.
    case post(key: ReaderSavedKey, identity: ReaderPostSaveIdentity?)
    /// The whole of a post's content.
    case content(key: ReaderSavedKey)
    case comments(siteID: UInt64, postID: Int64)
    case postLikes(siteID: UInt64, postID: Int64)
    case commentLikes(siteID: UInt64, commentID: Int64)
    case saved
    case subscriptions
    case blockedSites
    case findSites
}

extension ReaderRoute {
    static func post(_ post: ReaderPostRecord) -> ReaderRoute {
        .post(key: post.key, identity: post.identity)
    }
}

/// Opens a Reader screen from a row that can't hold a link, such as one with several buttons.
struct ReaderOpenAction: Sendable {
    let handler: @MainActor @Sendable (ReaderRoute) -> Void

    @MainActor
    func callAsFunction(_ route: ReaderRoute) {
        handler(route)
    }
}

extension EnvironmentValues {
    @Entry var openReaderRoute = ReaderOpenAction { _ in }
}
