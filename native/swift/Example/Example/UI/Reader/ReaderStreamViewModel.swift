import SwiftUI
import WordPressAPI
import WordPressApiCache

/// A row of a stream: a post, the place where posts are missing, or part of a card that isn't a post.
enum ReaderStreamEntry: Identifiable, Sendable {
    /// Posts may be missing above the post `below`.
    case gap(below: ReaderSavedKey)
    case post(ReaderPostRecord, isSaved: Bool)
    case cardTitle(id: String, title: String)
    case cardSite(id: String, site: ReaderStreamCardSite)
    case cardTag(id: String, tag: ReaderStreamCardTag)

    var id: String {
        switch self {
        case .gap(let below): "gap-\(below.rowID)"
        case .post(let post, _): post.key.rowID
        case .cardTitle(let id, _), .cardSite(let id, _), .cardTag(let id, _): id
        }
    }
}

/// What a site's, a feed's or a tag's stream shows above its posts.
struct ReaderStreamHeader {
    var title: String
    var description: String?
    var subscribers: UInt64?
    /// `nil` when it isn't known whether the reader follows it.
    var isFollowing: Bool?
}

/// A Reader stream from the cache, through the service's collection. The rows reload whenever the cache changes in
/// a way the stream shows, so a refresh, a like, a follow, a block and a relaunch all show up without any
/// bookkeeping here.
@MainActor
@Observable
final class ReaderStreamViewModel {
    let session: ReaderSession
    let stream: ReaderStream

    @ObservationIgnored private let collection: ReaderStreamCollection
    @ObservationIgnored private var watcher: ReaderCacheWatcher?
    @ObservationIgnored private var hasStarted = false
    @ObservationIgnored private var reloads = 0
    @ObservationIgnored private var site: ReaderSiteRecord?
    @ObservationIgnored private var tag: ReaderTag?

    private(set) var entries: [ReaderStreamEntry] = []
    private(set) var postCount = 0
    private(set) var info: ReaderStreamInfo?
    private(set) var header: ReaderStreamHeader?
    private(set) var followedSites: Set<String> = []
    private(set) var followedTags: Set<String> = []
    private(set) var notice: String?
    private var working = 0

    /// What went wrong loading the stream, shown in its status.
    var loadError: String?
    /// What went wrong with something the reader did to a post, shown as an alert.
    var actionError: String?

    init(session: ReaderSession, stream: ReaderStream) {
        self.session = session
        self.stream = stream
        self.collection = session.service.stream(stream: stream)
    }

    private var service: ReaderService { session.service }

    var isWorking: Bool { working > 0 }

    var hasMore: Bool { info?.hasMore ?? false }

    /// Shows what the cache has straight away, then fetches the first page.
    func start() async {
        guard !hasStarted else { return }
        hasStarted = true

        let collection = collection
        let savedPosts = service.saved().savedPosts()
        // A site's, a feed's and a tag's header shows whether the reader follows it, which no collection says is
        // relevant, so those streams reload on any change.
        let reloadsOnAnyChange = stream.hasHeader
        watcher = ReaderCacheWatcher(
            session.cache.databaseUpdatesPublisher()
                .filter { @Sendable hook in
                    reloadsOnAnyChange || collection.isRelevantUpdate(hook: hook)
                        || savedPosts.isRelevantUpdate(hook: hook)
                }
        ) { [weak self] in
            await self?.reloadFromCache()
        }

        await reloadFromCache()
        async let header: Void = refreshHeader()
        await refresh()
        await header
    }

    func refresh() async {
        await load { try await self.collection.refresh() }
    }

    func loadMore() async {
        await load { try await self.collection.loadMore() }
    }

    func fillGap(below: ReaderSavedKey) async {
        await load { try await self.collection.fillGap(below: below) }
    }

    func retryUnreadablePosts() async {
        working += 1
        defer { working -= 1 }
        do {
            let readable = try await collection.retryUnreadablePosts()
            notice = "\(readerCount(readable, "post")) that couldn't be read can be read now."
            loadError = nil
        } catch {
            loadError = readerErrorMessage(error)
        }
        await reloadFromCache()
    }

    // MARK: What the reader does to a post

    func setLiked(_ post: ReaderPostRecord, _ liked: Bool) async {
        await act(liked ? "Like" : "Unlike") {
            try await self.service.setPostLiked(post: post.identity, liked: liked).message
        }
    }

    func setSaved(_ post: ReaderPostRecord, _ saved: Bool) async {
        await act(saved ? "Save" : "Unsave") {
            // The stream has the post's record, not the API's post `saveWithDetails` takes.
            let service = self.service.saved()
            if saved {
                return try await service.save(post: post.identity).message
            }
            return try await service.unsave(post: post.identity).message
        }
    }

    func setSeen(_ post: ReaderPostRecord, _ seen: Bool) async {
        await act(seen ? "Mark as seen" : "Mark as unseen") {
            try await self.service.setPostSeen(post: post.identity, seen: seen).message
        }
    }

    func setFollowed(_ post: ReaderPostRecord, _ followed: Bool) async {
        await setFollowed(post.followTarget, followed)
    }

    func setFollowed(_ target: ReaderFollowTarget, _ followed: Bool) async {
        await act(followed ? "Follow" : "Unfollow") {
            try await self.service.setFollowed(target: target, followed: followed).message
        }
    }

    func blockSite(of post: ReaderPostRecord) async {
        guard let siteID = post.wordPressSiteID else { return }
        await act("Block site") {
            try await self.service.setSiteBlocked(siteId: siteID, blocked: true).message
        }
    }

    func blockAuthor(of post: ReaderPostRecord) async {
        guard let authorID = post.authorId, authorID != 0 else { return }
        await act("Block author") {
            try self.service.setUserBlocked(userId: authorID, blocked: true)
            return "Blocked on this device. WordPress.com keeps no list of blocked users."
        }
    }

    func setTagFollowed(slug: String, title: String?, _ followed: Bool) async {
        await act(followed ? "Follow tag" : "Unfollow tag") {
            try await self.service.setTagFollowed(slug: slug, title: title, followed: followed).message
        }
    }

    /// Follows or unfollows the site, feed or tag the stream is of.
    func setHeaderFollowed(_ followed: Bool) async {
        switch stream {
        case .tag(let slug):
            await setTagFollowed(slug: slug, title: tag?.title, followed)
        default:
            guard let site else { return }
            let address = site.siteId == nil ? (site.feedUrl ?? site.url) : (site.url ?? site.feedUrl)
            let target = ReaderFollowTarget(siteId: site.siteId, feedId: site.feedId, url: address, name: site.name)
            await setFollowed(target, followed)
        }
    }

    // MARK: Loading

    private func load(_ work: () async throws -> ReaderStreamPageResult) async {
        working += 1
        defer { working -= 1 }
        do {
            let page = try await work()
            loadError = nil
            notice =
                "Loaded \(readerCount(page.postCount, "post")), \(page.newPostCount) of them new"
                + (page.unreadablePostCount > 0 ? ", and \(page.unreadablePostCount) that couldn't be read." : ".")
        } catch {
            loadError = readerErrorMessage(error)
        }
        await reloadFromCache()
    }

    private func act(_ name: String, _ work: () async throws -> String) async {
        working += 1
        defer { working -= 1 }
        do {
            notice = "\(name): \(try await work())"
        } catch {
            actionError = "\(name) failed. \(readerErrorMessage(error))"
        }
        await reloadFromCache()
    }

    private func reloadFromCache() async {
        reloads += 1
        let reload = reloads
        do {
            let items = try await collection.loadItems()
            let cards = try await collection.loadCards()
            let entries = try await Self.entries(items: items, cards: cards, saved: service.saved())
            // A later reload has newer rows, and may already have shown them.
            guard reload == reloads else { return }

            self.entries = entries
            postCount = items.count
            info = try collection.info()
            followedTags = Set(try service.followedTags().compactMap(\.slug))
            if !cards.isEmpty {
                followedSites = Set(try service.subscriptions().flatMap(\.followKeys))
            }
            try reloadHeader()
        } catch {
            loadError = readerErrorMessage(error)
        }
    }

    /// The stream's rows: its posts in order, a gap row above a post that may have posts missing before it, and
    /// each card after the post it follows.
    nonisolated private static func entries(
        items: [ReaderStreamItem],
        cards: [ReaderStreamCard],
        saved: ReaderSavedService
    ) async throws -> [ReaderStreamEntry] {
        let keys = Set(items.map(\.post.key.rowID))
        var leading: [ReaderStreamEntry] = []
        var following: [String: [ReaderStreamEntry]] = [:]
        var trailing: [ReaderStreamEntry] = []
        for (position, card) in cards.enumerated() {
            let rows = card.card.rows(position: position)
            switch card.afterPost?.rowID {
            case .none: leading += rows
            case .some(let key) where keys.contains(key): following[key, default: []] += rows
            // The post the card follows isn't shown, such as a blocked site's.
            case .some: trailing += rows
            }
        }

        var entries = leading
        for item in items {
            if item.gapBefore {
                entries.append(.gap(below: item.post.key))
            }
            let isSaved = try saved.status(post: item.post.identity) == .saved
            entries.append(.post(item.post, isSaved: isSaved))
            entries += following[item.post.key.rowID] ?? []
        }
        return entries + trailing
    }

    // MARK: The header of a site's, a feed's or a tag's stream

    /// The header from the cache, which a follow or unfollow changes straight away.
    private func reloadHeader() throws {
        switch stream {
        case .site(let siteId):
            site = try service.cachedSite(siteId: siteId, feedId: nil) ?? site
        case .feed(let feedId):
            site = try service.cachedSite(siteId: nil, feedId: feedId) ?? site
        case .tag(let slug):
            header = ReaderStreamHeader(
                title: tag?.title?.decodingHTMLEntities() ?? slug,
                description: tag?.description?.readerPlainText,
                subscribers: nil,
                isFollowing: followedTags.contains(slug)
            )
            return
        default:
            return
        }
        if let site {
            header = ReaderStreamHeader(
                title: site.name?.decodingHTMLEntities() ?? site.url ?? stream.title,
                description: site.description?.readerPlainText,
                subscribers: site.subscribersCount,
                isFollowing: site.isFollowing
            )
        }
    }

    /// Fetches what the header shows. A tag's isn't cached, so it's kept here.
    private func refreshHeader() async {
        do {
            switch stream {
            case .site(let siteId): site = try await service.fetchSite(siteId: siteId)
            case .feed(let feedId): site = try await service.fetchFeed(feedId: feedId)
            case .tag(let slug): tag = try await service.tagInfo(slug: slug)
            default: return
            }
            try reloadHeader()
        } catch {
            loadError = "The stream's details couldn't be fetched. \(readerErrorMessage(error))"
        }
    }
}

extension ReaderStream {
    /// Whether the stream is of one site, feed or tag, which its screen shows above the posts.
    fileprivate var hasHeader: Bool {
        switch self {
        case .site, .feed, .tag: true
        default: false
        }
    }
}

extension ReaderStreamCardKind {
    /// The rows a card takes: its title, then a row for each site or tag.
    fileprivate func rows(position: Int) -> [ReaderStreamEntry] {
        let id = "card-\(position)"
        switch self {
        case .recommendedSites(let sites):
            return [.cardTitle(id: id, title: "Recommended sites")]
                + sites.enumerated().map { .cardSite(id: "\(id)-site-\($0.offset)", site: $0.element) }
        case .newSites(let sites):
            return [.cardTitle(id: id, title: "New sites")]
                + sites.enumerated().map { .cardSite(id: "\(id)-site-\($0.offset)", site: $0.element) }
        case .suggestedTags(let tags):
            return [.cardTitle(id: id, title: "Suggested tags")]
                + tags.enumerated().map { .cardTag(id: "\(id)-tag-\($0.offset)", tag: $0.element) }
        }
    }
}

extension ReaderSubscriptionRecord {
    /// The names a followed site or feed is known by, to match against a site on a card.
    var followKeys: [String] {
        [siteId.map { "site:\($0)" }, feedId.map { "feed:\($0)" }].compactMap { $0 }
    }
}

extension ReaderStreamCardSite {
    var followKeys: [String] {
        [siteId.map { "site:\($0)" }, feedId.map { "feed:\($0)" }].compactMap { $0 }
    }

    var followTarget: ReaderFollowTarget {
        ReaderFollowTarget(
            siteId: siteId,
            feedId: feedId,
            url: siteId == nil ? (feedUrl ?? url) : (url ?? feedUrl),
            name: name
        )
    }

    /// The site's stream, or its feed's when it has no site ID.
    var stream: ReaderStream? {
        if let siteId, siteId != 0 {
            return .site(siteId: siteId)
        }
        return feedId.map { .feed(feedId: $0) }
    }
}
