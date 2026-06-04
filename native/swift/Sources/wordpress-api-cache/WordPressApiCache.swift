import Foundation
import WordPressAPIInternal

#if canImport(Combine)
@preconcurrency import Combine
#endif

public final class WordPressApiCache: Sendable {
    package let cache: WpApiCache
    private let broadcaster: UpdateHookBroadcaster

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
        self.cache = try WpApiCache(path: path)
        self.broadcaster = UpdateHookBroadcaster()
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
    #endif

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

    deinit {
        self.cache.stopListeningForUpdates()

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
