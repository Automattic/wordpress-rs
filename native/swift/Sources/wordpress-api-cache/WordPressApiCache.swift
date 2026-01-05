import Foundation
import WordPressAPIInternal

public final class WordPressApiCache {

    let cache: WpApiCache

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
    }

    public func performMigrations() throws -> Int64 {
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
