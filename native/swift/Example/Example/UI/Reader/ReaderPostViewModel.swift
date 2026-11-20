import SwiftUI
import WordPressAPI
import WordPressApiCache

/// One post: shown from the cache straight away, then brought up to date from the server.
@MainActor
@Observable
final class ReaderPostViewModel {
    let session: ReaderSession
    let key: ReaderSavedKey

    /// What to fetch the post with when the cache doesn't have it.
    @ObservationIgnored private let identity: ReaderPostSaveIdentity?
    @ObservationIgnored private var watcher: ReaderCacheWatcher?
    @ObservationIgnored private var hasStarted = false

    private(set) var post: ReaderPostRecord?
    private(set) var isSaved = false
    private(set) var hasFetched = false
    /// What became of the view this opening may have counted.
    private(set) var pageView: String?
    /// What became of marking the post as seen when it was opened.
    private(set) var seenOnOpen: String?
    private(set) var notice: String?
    private var working = 0

    /// What went wrong fetching the post, shown in its status.
    var loadError: String?
    /// What went wrong with something the reader did, shown as an alert.
    var actionError: String?

    init(session: ReaderSession, key: ReaderSavedKey, identity: ReaderPostSaveIdentity?) {
        self.session = session
        self.key = key
        self.identity = identity
    }

    private var service: ReaderService { session.service }

    var isWorking: Bool { working > 0 }

    /// Shows the cached post, fetches it again, and reports that it was opened, which marks it as seen. The view
    /// is only counted when `countsPageViews` is on.
    func start(countsPageViews: Bool) async {
        guard !hasStarted else { return }
        hasStarted = true
        // No collection says which changes are this post's, so any change to the Reader's cache reloads it.
        watcher = ReaderCacheWatcher(session.cache.databaseUpdatesPublisher()) { [weak self] in
            self?.reloadFromCache()
        }
        reloadFromCache()
        await refresh()
        await reportOpened(countsPageViews: countsPageViews)
    }

    func refresh() async {
        guard let identity = post?.identity ?? identity else {
            loadError = "The cache doesn't have this post, and there's nothing to fetch it with."
            return
        }
        working += 1
        defer { working -= 1 }
        do {
            _ = try await service.fetchPost(post: identity)
            hasFetched = true
            loadError = nil
        } catch {
            loadError = readerErrorMessage(error)
        }
        reloadFromCache()
    }

    func setLiked(_ liked: Bool) async {
        await act(liked ? "Like" : "Unlike") { post in
            try await self.service.setPostLiked(post: post.identity, liked: liked).message
        }
    }

    func setSaved(_ saved: Bool) async {
        await act(saved ? "Save" : "Unsave") { post in
            // The screen has the post's record, not the API's post `saveWithDetails` takes.
            let service = self.service.saved()
            if saved {
                return try await service.save(post: post.identity).message
            }
            return try await service.unsave(post: post.identity).message
        }
    }

    func setSeen(_ seen: Bool) async {
        await act(seen ? "Mark as seen" : "Mark as unseen") { post in
            try await self.service.setPostSeen(post: post.identity, seen: seen).message
        }
    }

    func setFollowed(_ followed: Bool) async {
        await act(followed ? "Follow" : "Unfollow") { post in
            try await self.service.setFollowed(target: post.followTarget, followed: followed).message
        }
    }

    func setCommentsSubscribed(_ subscribed: Bool) async {
        await act(subscribed ? "Subscribe to comments" : "Unsubscribe from comments") { post in
            try await self.service.setCommentsSubscribed(post: post.identity, subscribed: subscribed).message
        }
    }

    func setCommentNotifications(_ enabled: Bool) async {
        await act(enabled ? "Turn comment notifications on" : "Turn comment notifications off") { post in
            try await self.service.setCommentNotifications(post: post.identity, enabled: enabled).message
        }
    }

    private func act(_ name: String, _ work: (ReaderPostRecord) async throws -> String) async {
        guard let post else { return }
        working += 1
        defer { working -= 1 }
        do {
            notice = "\(name): \(try await work(post))"
        } catch {
            actionError = "\(name) failed. \(readerErrorMessage(error))"
        }
        reloadFromCache()
    }

    private func reportOpened(countsPageViews: Bool) async {
        let choice: ReaderPageViewChoice =
            countsPageViews ? .count(userAgent: ReaderPageViewUserAgent.value) : .dontCount
        do {
            let opened = try await service.postOpened(key: key, pageView: choice)
            seenOnOpen = opened.seen.message
            pageView = opened.pageView.message
        } catch {
            let message = "Couldn't be reported. \(readerErrorMessage(error))"
            seenOnOpen = message
            pageView = message
        }
        reloadFromCache()
    }

    private func reloadFromCache() {
        do {
            if let post = try service.cachedPost(key: key) {
                self.post = post
                isSaved = try service.saved().status(post: post.identity) == .saved
            }
        } catch {
            loadError = readerErrorMessage(error)
        }
    }
}

extension ReaderOpenedSeen {
    /// What became of marking an opened post as seen, in words.
    var message: String {
        switch self {
        case .marked(let outcome): "Marked as seen. \(outcome.message)"
        case .alreadySeen: "It was seen already."
        case .noSeenState: "Not marked: the post has no seen state."
        case .notCached: "Not marked: the cache doesn't have the post."
        case .failed(let error): "Couldn't be marked. \(readerErrorMessage(error))"
        }
    }
}

extension ReaderOpenedPageView {
    /// What became of an opened post's view, in words.
    var message: String {
        switch self {
        case .reported(let outcome):
            switch outcome {
            case .counted: "Counted."
            case .notCountedForRssPost: "Not counted: an RSS post has no site stats."
            case .notCountedForSiteMember: "Not counted: the reader is a member of the post's site."
            case .notCountedForMissingDetails: "Not counted: the cache lacks the post, or its site's ID or URL."
            case .notCountedForUserAgent: "Not counted: the app has no User-Agent that stats count a view with."
            }
        case .notAsked: "Not sent: \"Count page views\" is off."
        case .failed(let error): "Couldn't be counted. \(readerErrorMessage(error))"
        }
    }
}
