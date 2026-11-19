package rs.wordpress.example.shared.ui.reader

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import rs.wordpress.cache.kotlin.createObservableCollection
import uniffi.wp_mobile_cache.UpdateHook

/** Runs [block] off the main thread: the cache is a file, and reading it can take a moment. */
internal suspend fun <T> io(block: suspend () -> T): T = withContext(Dispatchers.IO) { block() }

/**
 * What a screen's requests are doing, and what the last one said: the outcome of an action, or an
 * error, in words. A screen shows [message] where the reader will see it.
 */
@Stable
internal class ReaderRequests(private val scope: CoroutineScope) {
    private var running by mutableIntStateOf(0)

    var message by mutableStateOf<String?>(null)
        private set

    var isError by mutableStateOf(false)
        private set

    val isRunning: Boolean get() = running > 0

    /**
     * Runs [block] on the main thread, counted as a running request. An error it throws is shown
     * as the error. Inside it, wrap what reads the cache or the network in [io].
     */
    // Anything the request throws is shown to the reader, whatever it is, rather than lost.
    @Suppress("TooGenericExceptionCaught")
    fun launch(block: suspend () -> Unit) {
        scope.launch {
            running++
            try {
                block()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                failed(e)
            } finally {
                running--
            }
        }
    }

    /** Runs a request, off the main thread, and hands what it returns to [then], on the main thread. */
    fun <T> load(request: suspend () -> T, then: (T) -> Unit) = launch { then(io { request() }) }

    /** Runs a request, off the main thread, and shows what it returns as the last request's outcome. */
    fun run(request: suspend () -> String?) = load(request) { outcome -> outcome?.let { say(it) } }

    fun say(text: String) {
        message = text
        isError = false
    }

    fun failed(error: Throwable) {
        message = readerErrorText(error)
        isError = true
    }
}

@Composable
internal fun rememberReaderRequests(): ReaderRequests {
    val scope = rememberCoroutineScope()
    return remember(scope) { ReaderRequests(scope) }
}

/**
 * Calls [reload] now, and again each time the cache changes in a way [isRelevantUpdate] says the
 * screen shows. Changes that arrive while a reload runs become one more reload, so a page of posts
 * stored row by row is read once it's all there.
 */
// A failed reload is handed to `onError` to show, whatever it is, rather than lost.
@Suppress("TooGenericExceptionCaught")
@Composable
internal fun ReloadWhenCacheChanges(
    key: Any?,
    isRelevantUpdate: (UpdateHook) -> Boolean,
    onError: (Throwable) -> Unit,
    reload: suspend () -> Unit
) {
    val currentIsRelevantUpdate by rememberUpdatedState(isRelevantUpdate)
    val currentOnError by rememberUpdatedState(onError)
    val currentReload by rememberUpdatedState(reload)
    LaunchedEffect(key) {
        val changes = Channel<Unit>(Channel.CONFLATED)
        val observable = createObservableCollection(
            loadData = { emptyList<Unit>() },
            isRelevantUpdate = { currentIsRelevantUpdate(it) }
        )
        // The cache calls this on the thread that changed it, while that thread still holds the
        // cache, so it only leaves a note: reading the cache here would wait on itself.
        observable.addObserver { changes.trySend(Unit) }
        try {
            changes.send(Unit)
            for (ignored in changes) {
                try {
                    currentReload()
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    currentOnError(e)
                }
            }
        } finally {
            observable.close()
        }
    }
}

/**
 * Runs [block] the first time the screen opens, and not again when the reader comes back to it
 * from a screen it led to. A screen fetches this way, so coming back shows the list as it was.
 */
@Composable
internal fun OnFirstOpen(key: Any?, block: () -> Unit) {
    var opened by rememberSaveable { mutableStateOf(false) }
    LaunchedEffect(key) {
        if (!opened) {
            opened = true
            block()
        }
    }
}
