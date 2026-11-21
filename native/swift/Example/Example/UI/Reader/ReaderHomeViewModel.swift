import SwiftUI
import WordPressAPI
import WordPressApiCache

/// An entry of the Reader menu, as the home screen lists it.
struct ReaderMenuEntry: Identifiable {
    let id: String
    let title: String
    /// The stream the entry opens, or `nil` for one this app doesn't know how to open.
    let stream: ReaderStream?
}

/// A tag the home screen lists, with whether the reader follows it.
struct ReaderTagEntry: Identifiable {
    let slug: String
    let title: String
    let isFollowed: Bool

    var id: String { slug }
}

/// The Reader home: the menu, the tags and the waiting actions, all read from the cache and reloaded whenever
/// the cache changes.
@MainActor
@Observable
final class ReaderHomeViewModel {
    let session: ReaderSession

    @ObservationIgnored private var watcher: ReaderCacheWatcher?
    @ObservationIgnored private var hasStarted = false

    private(set) var streams: [ReaderMenuEntry] = []
    private(set) var lists: [ReaderMenuEntry] = []
    private(set) var followedTags: [ReaderTagEntry] = []
    private(set) var recommendedTags: [ReaderTagEntry] = []
    private(set) var interests: [ReaderTagEntry] = []
    private(set) var menuHasLoaded = false
    private(set) var pendingActions: UInt64 = 0
    private(set) var isWorking = false
    private(set) var notice: String?
    /// What went wrong fetching the menu, shown with the reader.
    var loadError: String?
    /// What went wrong with something the reader did, shown as an alert: the reader may be far down the list.
    var actionError: String?

    var tagToFollow = ""
    var searchQuery = ""
    var searchSort = ReaderSearchSort.relevance
    var siteID = ""
    var feedID = ""
    var postURL = ""
    /// Why the last link opened nothing, in words.
    private(set) var linkAnswer: String?
    var postSite = ""
    var postSlug = ""

    @ObservationIgnored private var loadedInterests: [ReaderInterest] = []

    init(session: ReaderSession) {
        self.session = session
    }

    private var service: ReaderService { session.service }

    /// The stream of the newest posts of every tag the reader follows, when they follow any.
    var followedTagsStream: ReaderStream? {
        followedTags.isEmpty ? nil : .tags(slugs: followedTags.map(\.slug))
    }

    /// Shows what the cache has straight away, then fetches the menu.
    func start() async {
        guard !hasStarted else { return }
        hasStarted = true
        // The home screen shows the menu, the tags and the waiting actions, which have no collection to ask
        // whether a change is theirs. Everything in this cache is the Reader's, so any change reloads.
        watcher = ReaderCacheWatcher(session.cache.databaseUpdatesPublisher()) { [weak self] in
            self?.reloadFromCache()
        }
        reloadFromCache()
        await refreshMenu()
    }

    func refreshMenu() async {
        isWorking = true
        defer { isWorking = false }
        do {
            try await service.refreshMenu()
            loadError = nil
        } catch {
            loadError = readerErrorMessage(error)
        }
    }

    /// Sends what's waiting: what the reader turned on or off, and their saves.
    func sync() async {
        isWorking = true
        defer { isWorking = false }

        var problems: [String] = []
        var waiting: UInt64?
        do {
            waiting = try await service.syncActions()
        } catch {
            problems.append(readerErrorMessage(error))
        }
        do {
            try await service.saved().sync()
        } catch {
            problems.append(readerErrorMessage(error))
        }
        notice = waiting.map { $0 == 0 ? "Everything is sent." : "\($0) still waiting to be sent." }
        actionError = problems.isEmpty ? nil : problems.joined(separator: "\n")
        reloadFromCache()
    }

    func setTagFollowed(slug: String, title: String?, followed: Bool) async {
        let slug = slug.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !slug.isEmpty else { return }
        await run {
            let outcome = try await self.service.setTagFollowed(slug: slug, title: title, followed: followed)
            self.notice = "\(followed ? "Followed" : "Unfollowed") \(slug). \(outcome.message)"
            if self.tagToFollow.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() == slug {
                self.tagToFollow = ""
            }
        }
    }

    func loadInterests() async {
        await run {
            self.loadedInterests = try await self.service.interests()
            self.reloadFromCache()
        }
    }

    /// Takes over, for this account, the tags followed and the posts saved while nobody was signed in.
    func adoptSignedOut() async {
        await run {
            let saves = try await self.service.saved().adoptSignedOutSaves()
            let tags = try await self.service.adoptSignedOutTags()
            let refusedTags = tags.refused.map {
                " The server refused to follow the tag \($0.title ?? $0.slug): \(readerErrorMessage($0.error))"
            }
            self.notice =
                "Took over \(readerCount(tags.adopted, "tag")). Saved posts: \(saves.imported) on the server, "
                + "\(saves.pending) waiting to be sent, \(saves.rejected.count) refused, "
                + "\(saves.waitingForRoom) waiting for room, \(saves.unsaveable) that can't be saved."
                + refusedTags.joined()
        }
    }

    /// Finds what the link in `postURL` opens, and returns the screen that shows it: a post, or a stream, such as
    /// a site's, a tag's or a search. A link the Reader can't show sets `linkAnswer` to why, and returns `nil`, as
    /// does the search page's address, which has no query.
    func openLink() async -> ReaderRoute? {
        let url = postURL.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !url.isEmpty else { return nil }
        linkAnswer = nil
        var route: ReaderRoute?
        await run {
            switch try await self.service.resolveLink(url: url) {
            case .post(let post):
                route = .post(post)
            case .site(let siteId, _):
                route = .stream(.site(siteId: siteId))
            case .feed(let feedId, _):
                route = .stream(.feed(feedId: feedId))
            case .stream(.discover(let tags)) where tags.isEmpty && self.session.isSignedOut:
                // Discover from an address names no tags: ask for the reader's, as the menu's Discover does.
                route = .stream(.discover(tags: self.followedTags.map(\.slug)))
            case .stream(let stream):
                route = .stream(stream)
            case .search:
                self.linkAnswer = "That's the Reader's search. Type what to search for under Search posts."
            case .notInReader(let reason):
                self.linkAnswer = reason.message
            }
        }
        return route
    }

    /// Fetches the post with the slug `postSlug` on the site `postSite`, and returns the screen that shows it.
    func openPostBySlug() async -> ReaderRoute? {
        let site = postSite.trimmingCharacters(in: .whitespacesAndNewlines)
        let slug = postSlug.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !site.isEmpty, !slug.isEmpty else { return nil }
        let identifier: WpComSiteIdentifier = UInt64(site).map { .id(value: $0) } ?? .slug(value: site)
        var route: ReaderRoute?
        await run {
            route = .post(try await self.service.fetchPostBySlug(site: identifier, slug: slug))
        }
        return route
    }

    private func reloadFromCache() {
        do {
            let menu = try service.menu()
            let followed = Set(menu.followedTags.compactMap(\.slug))
            let discoverTags = session.isSignedOut ? menu.followedTags.compactMap(\.slug) : []

            menuHasLoaded = !menu.items.isEmpty
            let entries = menu.items.map { item in
                (item.kind, ReaderMenuEntry(item: item, discoverTags: discoverTags))
            }
            streams = entries.filter { $0.0 != .list }.map(\.1)
            lists = entries.filter { $0.0 == .list }.map(\.1)
            followedTags = menu.followedTags.compactMap { ReaderTagEntry(item: $0, followed: followed) }
            recommendedTags = menu.recommendedTags.compactMap { ReaderTagEntry(item: $0, followed: followed) }
            interests = loadedInterests.map { interest in
                ReaderTagEntry(
                    slug: interest.slug,
                    title: interest.title?.decodingHTMLEntities() ?? interest.slug,
                    isFollowed: followed.contains(interest.slug)
                )
            }
            // What the reader turned on or off, and their saves, which the saved posts service counts.
            pendingActions = try service.pendingActionCount() + service.saved().pendingCount()
        } catch {
            loadError = readerErrorMessage(error)
        }
    }

    private func run(_ work: () async throws -> Void) async {
        isWorking = true
        defer { isWorking = false }

        do {
            try await work()
        } catch {
            actionError = readerErrorMessage(error)
        }
    }
}

extension ReaderMenuEntry {
    /// `discoverTags` are the reader's interests, which Discover is asked for when the server doesn't know them:
    /// for a reader who isn't signed in.
    init(item: ReaderMenuItem, discoverTags: [String]) {
        id = "\(item.kind)-\(item.key)"
        title = item.title?.decodingHTMLEntities() ?? item.key
        stream =
            switch item.stream {
            case .discover: .discover(tags: discoverTags)
            case let stream: stream
            }
    }
}

extension ReaderTagEntry {
    /// `nil` for a menu entry without a slug, which can't be opened or followed.
    init?(item: ReaderMenuItem, followed: Set<String>) {
        guard let slug = item.slug else { return nil }
        self.init(
            slug: slug,
            title: item.title?.decodingHTMLEntities() ?? slug,
            isFollowed: followed.contains(slug)
        )
    }
}
