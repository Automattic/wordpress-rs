package rs.wordpress.cache.kotlin

import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import uniffi.wp_mobile.QueuedActionEvent
import uniffi.wp_mobile.QueuedActionListener
import uniffi.wp_mobile.addQueuedActionListener
import uniffi.wp_mobile_cache.WpApiCache
import java.nio.file.Path

class WordPressApiCache {
    val cache: WpApiCache

    private val queuedActions = MutableSharedFlow<QueuedActionEvent>(
        extraBufferCapacity = QUEUED_ACTION_BUFFER,
        onBufferOverflow = BufferOverflow.DROP_OLDEST
    )

    /**
     * Every change to something the user turned on or off in this cache, such as a like or a
     * follow: when it starts waiting to be sent, when the server takes it, and when the server
     * refuses it. A change made offline and refused later is heard of only here.
     *
     * Events are in memory only, and an event sent while nobody collects is dropped. A collector
     * that falls more than 256 events behind loses the oldest of them.
     */
    val queuedActionEvents: SharedFlow<QueuedActionEvent> = queuedActions.asSharedFlow()

    // Creates a new in-memory cache
    constructor() : this(":memory:")

    // Creates a new cache at the specified file system URL
    constructor(path: Path) : this(path.toString())

    // Creates a new cache at the specified path
    constructor(string: String) {
        this.cache = WpApiCache(string)

        // Automatically register DatabaseChangeNotifier to enable ObservableEntity support
        this.cache.startListeningForUpdates(DatabaseChangeNotifier)
        addQueuedActionListener(cache, QueuedActionEmitter(queuedActions))
    }

    fun performMigrations(): Int {
        return cache.performMigrations().toInt()
    }

    private companion object {
        const val QUEUED_ACTION_BUFFER = 256
    }
}

/** Hands each event to `flow` without waiting, so the library is never held up by a collector. */
private class QueuedActionEmitter(
    private val flow: MutableSharedFlow<QueuedActionEvent>
) : QueuedActionListener {
    override fun queuedActionChanged(event: QueuedActionEvent) {
        flow.tryEmit(event)
    }
}
