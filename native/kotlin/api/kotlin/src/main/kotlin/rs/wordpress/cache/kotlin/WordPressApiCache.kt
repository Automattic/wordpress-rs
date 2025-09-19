package rs.wordpress.cache.kotlin

import kotlinx.coroutines.asCoroutineDispatcher
import uniffi.wp_api.DatabaseDelegate
import uniffi.wp_api.UpdateHook
import uniffi.wp_api.WpApiCache
import uniffi.wp_api.setGlobalDelegate
import java.nio.file.Path
import java.util.concurrent.Executors

class WordPressApiCacheDelegate : DatabaseDelegate {
    override fun didUpdate(updateHook: UpdateHook) {
        println("Received update: $updateHook")
    }
}

class WordPressApiCache {
    private val cache: WpApiCache
    private val internalDispatcher = Executors.newSingleThreadExecutor().asCoroutineDispatcher()
    private val delegate: DatabaseDelegate = WordPressApiCacheDelegate()

    // Creates a new in-memory cache
    constructor() : this(":memory:")

    // Creates a new cache at the specified file system URL
    constructor(path: Path) : this(path.toString())

    // Creates a new cache at the specified path
    constructor(string: String) {
        this.cache = WpApiCache(string)
    }

    fun performMigrations(): Int {
        internalDispatcher.run {
            return this@WordPressApiCache.cache.performMigrations().toInt()
        }
    }

    fun startListeningForUpdates() {
        setGlobalDelegate(delegate)
        this.cache.startListeningForUpdates()
    }

    fun stopListeningForUpdates() {
        this.cache.stopListeningForUpdates()
    }
}
