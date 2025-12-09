import Foundation
import WordPressAPIInternal

public actor WordPressApiCache {

    let cache: WpApiCache

    /// Creates a new in-memory cache
    public init() throws {
        try self.init(path: ":memory:")
    }

    /// Creates a new cache at the specified file system URL
    public init(url: URL) throws {
        try self.init(path: url.absoluteString)
    }

    /// Creates a new cache at the specified path
    public init(path: String) throws {
        self.cache = try WpApiCache(path: path)
    }

    public func performMigrations() async throws -> UInt64 {
        try self.cache.performMigrations()
    }

    public func startListeningForUpdates() {
        self.cache.startListeningForUpdates(delegate: DatabaseChangeNotifier.shared)
    }

    public func stopListeningForUpdates() {
        self.cache.stopListeningForUpdates()
    }

    deinit {
        self.cache.stopListeningForUpdates()
    }
}
