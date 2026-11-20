import Combine
import SwiftUI
import WordPressAPI
import WordPressApiCache

// MARK: - Errors and outcomes in words

/// What an error says, in words. The Reader's errors carry the library's own sentence; an API error has a
/// description of its own.
func readerErrorMessage(_ error: any Error) -> String {
    switch error {
    case let error as ReaderError:
        if case .Api(let apiError) = error {
            return apiError.localizedDescription
        }
        return error.message
    case let error as ReaderSavedError:
        if case .Api(let apiError) = error {
            return apiError.localizedDescription
        }
        return error.message
    default:
        return error.localizedDescription
    }
}

extension ReaderActionOutcome {
    /// What became of something the reader turned on or off, in words.
    var message: String {
        switch self {
        case .synced: "The server has it."
        case .pending(let reason): "Not sent yet. \(reason)"
        }
    }
}

extension ReaderSavedOutcome {
    var message: String {
        switch self {
        case .synced: "The server has it."
        case .pending(let reason): "Not sent yet. \(reason)"
        }
    }
}

// MARK: - Watching the cache

/// Calls `reload` on the main actor when `changes` sends something. A burst of changes, such as a page of posts
/// being stored, becomes one reload. It stops when it's released.
///
/// The cache sends its changes on whichever thread made them. A closure that filters them, such as one that asks a
/// collection whether a change is relevant, runs on that thread, so mark it `@Sendable`: a closure written in a
/// main-actor type is otherwise taken to run on the main actor, and stops the app when it doesn't.
@MainActor
final class ReaderCacheWatcher {
    private var cancellable: AnyCancellable?

    init<Changes: Publisher>(_ changes: Changes, reload: @escaping @MainActor () async -> Void)
    where Changes.Failure == Never {
        cancellable = Self.subscribe(to: changes, reload: reload)
    }

    /// Not on the main actor, so that the closures here aren't either: they run on the thread that changed the
    /// cache until the throttle moves the change to the main queue.
    nonisolated private static func subscribe<Changes: Publisher>(
        to changes: Changes,
        reload: @escaping @MainActor () async -> Void
    ) -> AnyCancellable where Changes.Failure == Never {
        changes
            .map { _ in () }
            .throttle(for: .milliseconds(50), scheduler: DispatchQueue.main, latest: true)
            .sink { _ in
                Task { @MainActor in await reload() }
            }
    }
}

// MARK: - What the screens show

extension ReaderSavedKey {
    /// A stable name for the post, to tell rows apart.
    var rowID: String {
        switch self {
        case .post(let blogId, let postId): "post:\(blogId):\(postId)"
        case .feedItem(let feedItemId): "feed-item:\(feedItemId)"
        case .url(let url): "url:\(url)"
        }
    }
}

extension ReaderStream {
    /// The stream's name, to show as a title.
    var title: String {
        simpleTitle ?? parameterizedTitle
    }

    private var simpleTitle: String? {
        switch self {
        case .following: "Following"
        case .liked: "Liked"
        case .automattic: "Automattic"
        case .p2: "P2"
        case .freshlyPressed: "Freshly Pressed"
        case .firstPosts: "First Posts"
        case .discover: "Discover"
        default: nil
        }
    }

    private var parameterizedTitle: String {
        switch self {
        case .site(let siteId): "Site \(siteId)"
        case .feed(let feedId): "Feed \(feedId)"
        case .tag(let slug): "Tag: \(slug)"
        case .tags(let slugs): "Latest from \(slugs.count) tags"
        case .search(let query, let sort, let lang):
            "Search: \(query) (\(sort == .date ? "newest" : "best match"), in \(lang ?? "no language"))"
        case .readerList(let owner, let slug): "List: \(owner)/\(slug)"
        case .related(let siteId, let postId): "Related to \(siteId)/\(postId)"
        default: "Stream"
        }
    }
}

extension ListState {
    var label: String {
        switch self {
        case .idle: "Idle"
        case .fetchingFirstPage: "Loading the first page…"
        case .fetchingNextPage: "Loading more…"
        case .error: "Failed"
        }
    }
}

extension ReaderPostCardType {
    var label: String {
        switch self {
        case .default: "default"
        case .photo: "photo"
        case .gallery: "gallery"
        case .video: "video"
        case .crossPost: "cross-post"
        }
    }
}

extension ReaderSubscriptionEmailFrequency {
    var label: String {
        switch self {
        case .instantly: "Instantly"
        case .daily: "Daily"
        case .weekly: "Weekly"
        case .other(let frequency): frequency
        }
    }
}

/// A count with what it counts: "1 like", "2 likes".
func readerCount(_ count: some BinaryInteger, _ noun: String) -> String {
    "\(count) \(noun)\(count == 1 ? "" : "s")"
}

extension Date {
    /// The date as the Reader screens show it.
    var readerLabel: String {
        formatted(date: .abbreviated, time: .shortened)
    }
}

extension String {
    /// HTML as plain text: tags removed, and character references decoded.
    var readerPlainText: String {
        replacingOccurrences(of: "<[^>]+>", with: "", options: .regularExpression)
            .decodingHTMLEntities()
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

extension ReaderPostRecord {
    /// The post's title, or its address when it has none.
    var displayTitle: String {
        if let title = title?.readerPlainText, !title.isEmpty {
            return title
        }
        return identity.url ?? key.rowID
    }

    /// What `setFollowed` needs to follow or unfollow the post's site or feed.
    var followTarget: ReaderFollowTarget {
        ReaderFollowTarget(
            siteId: identity.siteId,
            feedId: identity.feedId,
            url: identity.isExternal ? (feedUrl ?? siteUrl) : (siteUrl ?? feedUrl),
            name: siteName
        )
    }

    /// The site's ID, when the post belongs to a WordPress.com or Jetpack site.
    var wordPressSiteID: UInt64? {
        guard !identity.isExternal, let siteId = identity.siteId, siteId != 0 else { return nil }
        return siteId
    }
}

// MARK: - Small views

/// What the last action did, such as "Waiting to be sent".
struct ReaderNoticeRow: View {
    let message: String

    var body: some View {
        Text(message)
            .font(.callout)
            .foregroundStyle(.secondary)
            .accessibilityIdentifier("reader-notice")
    }
}

/// A short word in a rounded box, such as "Seen" or "Saved".
struct ReaderBadge: View {
    let text: String

    init(_ text: String) {
        self.text = text
    }

    var body: some View {
        Text(text)
            .font(.caption2.weight(.semibold))
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(.quaternary, in: Capsule())
    }
}

extension View {
    /// A text field for an address, a slug or a search: nothing capitalized for the reader.
    func readerPlainInput() -> some View {
        #if os(macOS)
        self
        #else
        self.textInputAutocapitalization(.never)
        #endif
    }

    /// A text field for an ID.
    func readerNumberInput() -> some View {
        #if os(iOS)
        self.keyboardType(.numberPad)
        #else
        self
        #endif
    }
}
