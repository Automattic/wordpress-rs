package rs.wordpress.cache.kotlin

import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.withContext
import uniffi.wp_api.DatabaseDelegate
import uniffi.wp_api.UpdateHook
import uniffi.wp_api.WpApiCache
import java.nio.file.Path
import java.util.concurrent.Executors

class WordPressApiCacheLoggingDelegate : DatabaseDelegate {
    override fun didUpdate(updateHook: UpdateHook) {
        println("Received update: $updateHook")
    }
}
class WordPressApiCacheDelegate(
    private val callback: (updateHook: UpdateHook) -> Unit
) : DatabaseDelegate {

    override fun didUpdate(updateHook: UpdateHook) {
        callback(updateHook)
    }
}

class WordPressApiCache {
    private val cache: WpApiCache
    private val internalDispatcher = Executors.newSingleThreadExecutor().asCoroutineDispatcher()
    private val delegate: DatabaseDelegate?

    // Creates a new in-memory cache
    constructor(delegate: WordPressApiCacheDelegate? = null) : this(":memory:", delegate)

    // Creates a new cache at the specified file system URL
    constructor(path: Path, delegate: WordPressApiCacheDelegate? = null) : this(path.toString(), delegate)

    // Creates a new cache at the specified path
    constructor(string: String, delegate: WordPressApiCacheDelegate? = null) {
        this.cache = WpApiCache(string)
        this.delegate = delegate
    }

    suspend fun performMigrations(): Int = withContext(internalDispatcher) {
        cache.performMigrations().toInt()
    }
    fun startListeningForUpdates() {
        if (this.delegate != null) {
            this.cache.startListeningForUpdates(this.delegate)
        }
    }

    fun stopListeningForUpdates() {
        this.cache.stopListeningForUpdates()
    }
}
