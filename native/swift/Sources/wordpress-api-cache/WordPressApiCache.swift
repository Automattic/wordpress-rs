import Foundation
import WordPressAPIInternal

#if canImport(Combine)
@preconcurrency import Combine
#endif

/// A change to the cache: which table, which row, and what happened to it.
public typealias UpdateHook = WordPressAPIInternal.UpdateHook
/// A table of the cache.
public typealias DbTable = WordPressAPIInternal.DbTable
/// What a change to the cache did to a row.
public typealias HookAction = WordPressAPIInternal.HookAction
/// A change to something the user turned on or off, such as a like or a follow: it waits to be
/// sent, the server took it, or the server refused it.
public typealias QueuedActionEvent = WordPressAPIInternal.QueuedActionEvent
/// Something the user turns on or off, with the IDs that say what it's about.
public typealias QueuedAction = WordPressAPIInternal.QueuedAction
/// What became of a change the user made.
public typealias QueuedActionState = WordPressAPIInternal.QueuedActionState

public final class WordPressApiCache: Sendable {
    package let cache: WpApiCache
    private let broadcaster: UpdateHookBroadcaster
    private let queuedActionBroadcaster: QueuedActionBroadcaster
    private let queuedActionListenerID: UInt64

    #if canImport(Combine)
    private let stopUpdatesNotifier = PassthroughSubject<Void, Never>()
    #endif

    /// Creates a new in-memory cache
    public convenience init() throws {
        try self.init(path: ":memory:")
    }

    /// Creates a new cache at the specified file system URL
    public convenience init(url: URL) throws {
        try self.init(path: url.absoluteString)
    }

    /// Creates a new cache at the specified path
    public init(path: String) throws {
        let cache = try WpApiCache(path: path)
        let queuedActionBroadcaster = QueuedActionBroadcaster()
        self.cache = cache
        self.broadcaster = UpdateHookBroadcaster()
        self.queuedActionBroadcaster = queuedActionBroadcaster
        self.queuedActionListenerID = addQueuedActionListener(cache: cache, listener: queuedActionBroadcaster)
        queuedActionBroadcaster.attach(self)
    }

    public func performMigrations() throws -> Int64 {
        try self.cache.performMigrations()
    }

    public func startListeningForUpdates() {
        self.cache.startListeningForUpdates(delegate: self.broadcaster)
    }

    public func stopListeningForUpdates() {
        self.cache.stopListeningForUpdates()
    }

    public func addDatabaseUpdatesObserver(_ body: @Sendable @escaping (UpdateHook) -> Void) -> NSObjectProtocol {
        NotificationCenter.default.addObserver(
            forName: UpdateHookBroadcaster.notification,
            object: broadcaster,
            queue: nil
        ) {
            guard let notification = UpdateHookNotification(from: $0) else { return }
            body(notification.hook)
        }
    }

    #if canImport(Combine)
    /// The cache's changes, as they happen.
    ///
    /// A change is sent on the thread that made it, not on the main one. In Swift 6, a closure written inside a
    /// `@MainActor` type is taken to run on the main actor, and stops the app when it runs elsewhere: mark a
    /// closure that handles a change here, such as a `filter`, as `@Sendable`, or move to the main queue first
    /// with `receive(on:)`.
    public func databaseUpdatesPublisher() -> AnyPublisher<UpdateHook, Never> {
        NotificationCenter.default.publisher(for: UpdateHookBroadcaster.notification, object: broadcaster)
            .prefix(untilOutputFrom: stopUpdatesNotifier)
            .compactMap { UpdateHookNotification(from: $0)?.hook }
            .eraseToAnyPublisher()
    }

    @available(macOS 15.0, iOS 18.0, watchOS 11.0, tvOS 18.0, visionOS 2.0, *)
    public func databaseUpdates() -> some AsyncSequence<UpdateHook, Never> {
        databaseUpdatesPublisher().values
    }

    /// The changes to what the user turned on or off in this cache, as `addQueuedActionObserver(_:)`
    /// describes them.
    ///
    /// An event is sent on the thread that made the change, not on the main one: see
    /// `databaseUpdatesPublisher()`.
    public func queuedActionEventsPublisher() -> AnyPublisher<QueuedActionEvent, Never> {
        NotificationCenter.default.publisher(for: QueuedActionBroadcaster.notification)
            .prefix(untilOutputFrom: stopUpdatesNotifier)
            .compactMap { [weak self] in self?.queuedActionEvent(from: $0) }
            .eraseToAnyPublisher()
    }

    @available(macOS 15.0, iOS 18.0, watchOS 11.0, tvOS 18.0, visionOS 2.0, *)
    public func queuedActionEvents() -> some AsyncSequence<QueuedActionEvent, Never> {
        queuedActionEventsPublisher().values
    }
    #endif

    /// Calls `body` with every change to something the user turned on or off in this cache, such as
    /// a like or a follow: when it starts waiting to be sent, when the server takes it, and when
    /// the server refuses it. A change made offline and refused later is heard of only here.
    ///
    /// `body` is called on the thread that made the change, not on the main one, while the
    /// account's changes are being sent, so it must return quickly. Events are in memory only: one
    /// sent before an observer is added isn't sent to it later. Remove the observer with
    /// `NotificationCenter.default.removeObserver(_:)`.
    public func addQueuedActionObserver(
        _ body: @Sendable @escaping (QueuedActionEvent) -> Void
    ) -> NSObjectProtocol {
        NotificationCenter.default.addObserver(
            forName: QueuedActionBroadcaster.notification,
            object: nil,
            queue: nil
        ) { [weak self] notification in
            guard let event = self?.queuedActionEvent(from: notification) else { return }
            body(event)
        }
    }

    /// The event in `notification`, if it's one of this cache's.
    ///
    /// Observers filter by name only and check the `object` here: on Linux, an observer that names
    /// an `object` that isn't an `NSObject` hears nothing.
    private func queuedActionEvent(from notification: Notification) -> QueuedActionEvent? {
        guard (notification.object as? WordPressApiCache) === self else { return nil }
        return QueuedActionBroadcaster.event(from: notification)
    }

    /// Remove a self-hosted site and all its cached data from the database.
    ///
    /// Returns `true` if the site was found and removed, `false` if no site
    /// with the given URL exists.
    @discardableResult
    public func removeSelfHostedSite(url: URL) throws -> Bool {
        let parsed = try ParsedUrl.parse(input: url.absoluteString)
        return try self.cache.removeSelfHostedSite(url: parsed)
    }

    /// Remove a WordPress.com site and all its cached data from the database.
    ///
    /// Returns `true` if the site was found and removed, `false` if no site
    /// with the given site ID exists.
    @discardableResult
    public func removeWordpressComSite(siteId: WpComSiteId) throws -> Bool {
        try self.cache.removeWordpressComSite(siteId: siteId)
    }

    /// Remove a WordPress.com account and everything cached for it, such as its Reader posts and
    /// saved posts. Call it when the user signs out.
    ///
    /// Returns `true` if the account was found and removed, `false` if nothing was cached for it.
    @discardableResult
    public func removeWordpressComAccount(userId: WpComUserId) throws -> Bool {
        try self.cache.removeWordpressComAccount(userId: userId)
    }

    deinit {
        self.cache.stopListeningForUpdates()
        _ = removeQueuedActionListener(cache: self.cache, listenerId: self.queuedActionListenerID)

        #if canImport(Combine)
        stopUpdatesNotifier.send()
        #endif
    }
}

// Note: Inhering from `NSObject` is necessary for the `addObserver(..., object: ...)` to work properly on Linux.
// https://github.com/swiftlang/swift-corelibs-foundation/issues/3218
private final class UpdateHookBroadcaster: NSObject, DatabaseDelegate {
    static let notification = NSNotification.Name(rawValue: "wordpress-rs.UpdateHookBroadcaster")

    func didUpdate(updateHook: UpdateHook) {
        UpdateHookNotification(hook: updateHook).post(name: Self.notification, object: self)
    }
}

private struct UpdateHookNotification {
    var hook: UpdateHook

    private var userInfo: [AnyHashable: Any] {
        ["hook": self.hook]
    }

    init(hook: UpdateHook) {
        self.hook = hook
    }

    init?(from notification: Notification) {
        guard let hook = (notification.userInfo?["hook"] as? UpdateHook) else { return nil }
        self.init(hook: hook)
    }

    func post(name: Notification.Name, object: Any) {
        NotificationCenter.default.post(name: name, object: object, userInfo: userInfo)
    }
}

/// Posts each event of the cache it's attached to as a notification, whose `object` is the cache.
private final class QueuedActionBroadcaster: QueuedActionListener, @unchecked Sendable {
    static let notification = Notification.Name(rawValue: "wordpress-rs.QueuedActionChanged")
    static let eventKey = "event"

    private let lock = NSLock()
    // Weak, so the cache's `deinit` runs, and removes this listener.
    private weak var subject: WordPressApiCache?

    func attach(_ subject: WordPressApiCache) {
        lock.lock()
        defer { lock.unlock() }
        self.subject = subject
    }

    func queuedActionChanged(event: QueuedActionEvent) {
        lock.lock()
        let subject = self.subject
        lock.unlock()
        guard let subject else { return }
        NotificationCenter.default.post(name: Self.notification, object: subject, userInfo: [Self.eventKey: event])
    }

    static func event(from notification: Notification) -> QueuedActionEvent? {
        notification.userInfo?[eventKey] as? QueuedActionEvent
    }
}

extension QueuedActionEvent {
    /// A message to show the user for the event, in their preferred language, or in English until
    /// it's translated. `nil` when there's nothing to tell them: only a refused like or follow has
    /// one.
    public var userMessage: String? {
        userMessage(languages: Locale.preferredLanguages)
    }

    /// A message to show the user in the first of `languages` that has translations, or `nil`.
    public func userMessage(languages: [String]) -> String? {
        queuedActionEventUserMessage(event: self, locale: wpLocaleResolve(langIds: languages))
    }
}

#if canImport(Darwin)
@available(macOS 26, iOS 26, tvOS 26, watchOS 26, visionOS 26, *)
extension WordPressApiCache {
    /// A change to something the user turned on or off in a cache, as a typed notification
    /// message. Observe it with `NotificationCenter.default.addObserver(of: cache, for:
    /// .queuedActionChanged)` or `NotificationCenter.default.messages(of: cache, for:
    /// .queuedActionChanged)`.
    ///
    /// It's the notification that `addQueuedActionObserver(_:)` hears, so both kinds of observer
    /// hear one post. It's posted on the thread that made the change.
    public struct QueuedActionChangedMessage: NotificationCenter.AsyncMessage {
        public typealias Subject = WordPressApiCache

        public static var name: Notification.Name { QueuedActionBroadcaster.notification }

        public let event: QueuedActionEvent

        public init(event: QueuedActionEvent) {
            self.event = event
        }

        public static func makeMessage(_ notification: Notification) -> Self? {
            QueuedActionBroadcaster.event(from: notification).map(Self.init(event:))
        }

        public static func makeNotification(_ message: Self) -> Notification {
            Notification(name: name, userInfo: [QueuedActionBroadcaster.eventKey: message.event])
        }
    }
}

@available(macOS 26, iOS 26, tvOS 26, watchOS 26, visionOS 26, *)
extension NotificationCenter.MessageIdentifier
where Self == NotificationCenter.BaseMessageIdentifier<WordPressApiCache.QueuedActionChangedMessage> {
    /// A change to something the user turned on or off in a cache: see
    /// `WordPressApiCache.QueuedActionChangedMessage`.
    public static var queuedActionChanged: Self { .init() }
}
#endif
