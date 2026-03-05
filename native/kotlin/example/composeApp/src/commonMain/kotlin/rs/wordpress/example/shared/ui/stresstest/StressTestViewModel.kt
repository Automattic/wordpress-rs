package rs.wordpress.example.shared.ui.stresstest

import androidx.lifecycle.ViewModel
import kotlinx.coroutines.Dispatchers
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.cache.kotlin.ObservableCollection
import rs.wordpress.cache.kotlin.WordPressApiCache
import rs.wordpress.cache.kotlin.getObservablePostCollectionWithEditContext
import uniffi.wp_mobile.AnyPostFilter
import uniffi.wp_mobile.MockPostService
import uniffi.wp_mobile.StressTestHandle
import uniffi.wp_mobile.WpService
import kotlin.math.roundToInt

data class PerformanceMetrics(
    val avgLoadTimeMs: Int,
    val minLoadTimeMs: Long,
    val maxLoadTimeMs: Long,
    val avgTotalLatencyMs: Int,
    val last10LoadTimes: List<Long>,
    val sampleCount: Int
)

class StressTestViewModel(
    private val mockPostService: MockPostService,
    private val selfHostedService: WpService,
    private val cache: WordPressApiCache
) : ViewModel() {

    private val _posts = MutableStateFlow<List<PostDisplayData>>(emptyList())
    val posts: StateFlow<List<PostDisplayData>> = _posts.asStateFlow()

    private val _totalUpdates = MutableStateFlow(0L)
    val totalUpdates: StateFlow<Long> = _totalUpdates.asStateFlow()

    private val _totalInserts = MutableStateFlow(0L)
    val totalInserts: StateFlow<Long> = _totalInserts.asStateFlow()

    private val _totalDeletes = MutableStateFlow(0L)
    val totalDeletes: StateFlow<Long> = _totalDeletes.asStateFlow()

    private val _isRunning = MutableStateFlow(false)
    val isRunning: StateFlow<Boolean> = _isRunning.asStateFlow()

    private val _performanceMetrics = MutableStateFlow<PerformanceMetrics?>(null)
    val performanceMetrics: StateFlow<PerformanceMetrics?> = _performanceMetrics.asStateFlow()

    private var stressTestHandle: StressTestHandle? = null
    private var observableCollection: ObservableCollection<*>? = null

    // Performance tracking
    private val enableMetrics = true  // Set to false to disable metrics tracking overhead
    private val metricsLock = Any()
    private val recentLoadTimes = mutableListOf<Long>()
    private val recentTotalLatencies = mutableListOf<Long>()
    private val maxSamples = 100
    private var minLoadTime = Long.MAX_VALUE
    private var maxLoadTime = 0L

    private data class MetricsSnapshot(
        val size: Int,
        val avgLoad: Int,
        val avgLatency: Int,
        val last10: List<Long>,
        val minLoad: Long,
        val maxLoad: Long
    )

    init {
        // Run stress test initialization in background to avoid blocking main thread
        viewModelScope.launch(Dispatchers.IO) {
            startStressTest()
        }
    }

    private suspend fun startStressTest() {
        println("Starting stress test...")

        // Generate 1000 posts (runs on background thread)
        val entityIds = mockPostService.generateAndInsertPosts(1000u)
        println("Generated ${entityIds.size} posts")

        val postService = selfHostedService.posts()

        // Create a single observable collection for all posts
        val collection = postService.getObservablePostCollectionWithEditContext(AnyPostFilter())

        // Helper function to reload and update posts
        suspend fun reloadPostsAndMeasure(): Long {
            return try {
                val loadStartTime = System.currentTimeMillis()
                val allPosts = collection.loadData()
                val loadEndTime = System.currentTimeMillis()
                val loadDuration = loadEndTime - loadStartTime

                val postDataList = allPosts.map { fullEntity ->
                    PostDisplayData(
                        entityId = fullEntity.entityId,
                        title = fullEntity.data.title?.rendered ?: "<no-title>",
                        contentPreview = fullEntity.data.content.rendered.take(100),
                        status = fullEntity.data.status.toString(),
                        author = fullEntity.data.author?.toString(),
                        date = fullEntity.data.date,
                        modified = fullEntity.data.modified
                    )
                }

                _posts.value = postDataList
                loadDuration
            } catch (e: Exception) {
                println("ERROR loading posts: ${e.message}")
                e.printStackTrace()
                0L
            }
        }

        // Load initial data
        println("Loading initial data...")
        val initialLoadTime = reloadPostsAndMeasure()
        println("⏱️ Initial load: ${initialLoadTime}ms (${_posts.value.size} posts)")

        // Add observer to reload all posts when any change occurs
        collection.addObserver {
            val observerTriggerTime = System.currentTimeMillis()

            viewModelScope.launch(Dispatchers.IO) {
                // Measure DB load time
                val loadDuration = reloadPostsAndMeasure()

                // Calculate total latency (observer trigger → StateFlow update)
                val totalLatency = System.currentTimeMillis() - observerTriggerTime

                // Update performance metrics
                updatePerformanceMetrics(loadDuration, totalLatency)
            }
        }

        observableCollection = collection
        _isRunning.value = true

        println("Starting comprehensive stress test for ${entityIds.size} posts...")
        // Start comprehensive stress test with:
        // - 10-100ms delay between batches (variable timing)
        // - 1-20 posts per batch (variable batch size)
        // - 50% updates, 25% deletes, 25% inserts (operation weights)
        stressTestHandle = mockPostService.startComprehensiveStressTest(
            entityIds,
            uniffi.wp_mobile.StressTestConfig(
                minDelayMs = 10u,
                maxDelayMs = 100u,
                minBatchSize = 1u,
                maxBatchSize = 20u,
                updateWeight = 50u,
                deleteWeight = 25u,
                insertWeight = 25u
            )
        )
        println("Comprehensive stress test started with ObservableCollection (updates/deletes/inserts)!")

        // Poll operation counters from the stress test handle
        viewModelScope.launch(Dispatchers.Default) {
            while (_isRunning.value) {
                stressTestHandle?.let { handle ->
                    _totalUpdates.value = handle.updateCount().toLong()
                    _totalInserts.value = handle.insertCount().toLong()
                    _totalDeletes.value = handle.deleteCount().toLong()
                }
                kotlinx.coroutines.delay(100) // Poll every 100ms
            }
        }
    }

    private fun updatePerformanceMetrics(loadDuration: Long, totalLatency: Long) {
        if (!enableMetrics) return

        // Minimize work inside synchronized block - only mutate collections
        val snapshot = synchronized(metricsLock) {
            recentLoadTimes.add(loadDuration)
            if (recentLoadTimes.size > maxSamples) {
                recentLoadTimes.removeAt(0)
            }

            recentTotalLatencies.add(totalLatency)
            if (recentTotalLatencies.size > maxSamples) {
                recentTotalLatencies.removeAt(0)
            }

            if (loadDuration < minLoadTime) minLoadTime = loadDuration
            if (loadDuration > maxLoadTime) maxLoadTime = loadDuration

            // Calculate averages and copy data inside lock, but minimize work
            MetricsSnapshot(
                size = recentLoadTimes.size,
                avgLoad = recentLoadTimes.average().roundToInt(),
                avgLatency = recentTotalLatencies.average().roundToInt(),
                last10 = recentLoadTimes.takeLast(10),
                minLoad = minLoadTime,
                maxLoad = maxLoadTime
            )
        }

        // Everything else outside the lock to avoid contention
        _performanceMetrics.value = PerformanceMetrics(
            avgLoadTimeMs = snapshot.avgLoad,
            minLoadTimeMs = snapshot.minLoad,
            maxLoadTimeMs = snapshot.maxLoad,
            avgTotalLatencyMs = snapshot.avgLatency,
            last10LoadTimes = snapshot.last10,
            sampleCount = snapshot.size
        )

        // Log every 10 updates (outside lock)
        if (snapshot.size % 10 == 0) {
            println("⏱️ Performance (${snapshot.size} samples): " +
                    "avg=${snapshot.avgLoad}ms, min=${snapshot.minLoad}ms, max=${snapshot.maxLoad}ms, " +
                    "latency=${snapshot.avgLatency}ms")
        }
    }

    override fun onCleared() {
        // Stop the running flag to stop polling
        _isRunning.value = false

        // Stop background updates
        stressTestHandle?.stop()

        // Close the observable collection to unregister from DatabaseChangeNotifier
        observableCollection?.close()
        observableCollection = null
    }
}
