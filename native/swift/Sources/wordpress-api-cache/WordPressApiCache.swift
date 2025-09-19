import Foundation
import WordPressAPIInternal

public actor WordPressApiCache {

    private let cache: WpApiCache
    private let delegate: any DatabaseDelegate

    public struct Notifications {
        public static let cacheDidUpdate = Notification.Name("WordPressApiCache.cacheDidUpdate")

        public static func name(for table: String) -> Notification.Name {
            Notification.Name(rawValue: "WordPressApiCachce.cacheDidUpdate.\(table)")
        }
    }

    final public class ApiCacheDelegate: DatabaseDelegate {
        public init() {}

        public func didUpdate(updateHook: WordPressAPIInternal.UpdateHook) {
            let name = Notifications.name(for: updateHook.tableName)
            NotificationCenter.default.post(name: name, object: updateHook)
        }
    }

    /// Creates a new in-memory cache
    public init(delegate: DatabaseDelegate = ApiCacheDelegate()) throws {
        try self.init(path: ":memory:", delegate: delegate)
    }

    /// Creates a new cache at the specified file system URL
    public init(url: URL, delegate: DatabaseDelegate = ApiCacheDelegate()) throws {
        try self.init(path: url.absoluteString, delegate: delegate)
    }

    /// Creates a new cache at the specified path
    public init(path: String, delegate: DatabaseDelegate = ApiCacheDelegate()) throws {
        self.cache = try WpApiCache(path: path)
        self.delegate = delegate
    }

    public func performMigrations() async throws -> Int {
        return Int(try self.cache.performMigrations())
    }

    public func startListeningForUpdates() {
        setGlobalDelegate(delegate: delegate)
        self.cache.startListeningForUpdates()
    }

    public func stopListeningForUpdates() {
        self.cache.stopListeningForUpdates()
    }

    deinit {
        self.cache.stopListeningForUpdates()
    }
}
