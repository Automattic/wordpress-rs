package rs.wordpress.cache.kotlin

import uniffi.wp_mobile_cache.WpApiCache
import java.nio.file.Path

class WordPressApiCache {
    val cache: WpApiCache

    // Creates a new in-memory cache
    constructor() : this(":memory:")

    // Creates a new cache at the specified file system URL
    constructor(path: Path) : this(path.toString())

    // Creates a new cache at the specified path
    constructor(string: String) {
        this.cache = WpApiCache(string)

        // Automatically register DatabaseChangeNotifier to enable ObservableEntity support
        this.cache.startListeningForUpdates(DatabaseChangeNotifier)
    }

    fun performMigrations(): Int {
        return cache.performMigrations().toInt()
    }
}
