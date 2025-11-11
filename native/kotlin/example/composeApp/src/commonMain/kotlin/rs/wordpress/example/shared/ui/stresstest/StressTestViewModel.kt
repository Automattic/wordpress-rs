package rs.wordpress.example.shared.ui.stresstest

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import rs.wordpress.cache.kotlin.ObservableCollection
import rs.wordpress.cache.kotlin.WordPressApiCache
import rs.wordpress.cache.kotlin.getObservableAllPostsWithEditContext
import uniffi.wp_mobile.MockPostService
import uniffi.wp_mobile.StressTestHandle
import uniffi.wp_mobile.WpSelfHostedService
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
    private val selfHostedService: WpSelfHostedService,
    private val cache: WordPressApiCache
) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _posts = MutableStateFlow<List<PostDisplayData>>(emptyList())
    val posts: StateFlow<List<PostDisplayData>> = _posts.asStateFlow()

    private val _totalUpdates = MutableStateFlow(0L)
    val totalUpdates: StateFlow<Long> = _totalUpdates.asStateFlow()

    private val _isRunning = MutableStateFlow(false)
    val isRunning: StateFlow<Boolean> = _isRunning.asStateFlow()

    private val _performanceMetrics = MutableStateFlow<PerformanceMetrics?>(null)
    val performanceMetrics: StateFlow<PerformanceMetrics?> = _performanceMetrics.asStateFlow()

    private var stressTestHandle: StressTestHandle? = null
    private var observableCollection: ObservableCollection<*>? = null

    // Performance tracking
    private val recentLoadTimes = mutableListOf<Long>()
    private val recentTotalLatencies = mutableListOf<Long>()
    private val maxSamples = 100
    private var minLoadTime = Long.MAX_VALUE
    private var maxLoadTime = 0L

    init {
        startStressTest()
    }

    private fun startStressTest() {
        println("Starting stress test...")

        // Generate 1000 posts
        val entityIds = mockPostService.generateAndInsertPosts(1000u)
        println("Generated ${entityIds.size} posts")

        val postService = selfHostedService.posts()

        // Create a single observable collection for all posts
        val collection = postService.getObservableAllPostsWithEditContext()

        // Helper function to reload and update posts
        fun reloadPostsAndMeasure(): Long {
            return try {
                val loadStartTime = System.currentTimeMillis()
                val allPosts = collection.loadData()
                val loadEndTime = System.currentTimeMillis()
                val loadDuration = loadEndTime - loadStartTime

                val postDataList = allPosts.map { fullEntity ->
                    PostDisplayData(
                        entityId = fullEntity.entityId,
                        title = fullEntity.data.title.rendered,
                        contentPreview = fullEntity.data.content.rendered.take(100),
                        status = fullEntity.data.status.toString(),
                        author = fullEntity.data.author?.toString(),
                        date = fullEntity.data.date,
                        modified = fullEntity.data.modified,
                        updateCount = 0  // Not tracking per-post updates anymore
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

                // Increment total updates counter
                _totalUpdates.value += 1
            }
        }

        observableCollection = collection
        _isRunning.value = true

        println("Starting comprehensive stress test for ${entityIds.size} posts...")
        // Start comprehensive stress test with:
        // - 10-100ms delay between batches (variable timing)
        // - 1-20 posts per batch (variable batch size)
        stressTestHandle = mockPostService.startComprehensiveStressTest(
            entityIds,
            minDelayMs = 10u,
            maxDelayMs = 100u,
            minBatchSize = 1u,
            maxBatchSize = 20u
        )
        println("Comprehensive stress test started with ObservableCollection!")
    }

    private fun updatePerformanceMetrics(loadDuration: Long, totalLatency: Long) {
        // Track load times
        recentLoadTimes.add(loadDuration)
        if (recentLoadTimes.size > maxSamples) {
            recentLoadTimes.removeAt(0)
        }

        // Track total latencies
        recentTotalLatencies.add(totalLatency)
        if (recentTotalLatencies.size > maxSamples) {
            recentTotalLatencies.removeAt(0)
        }

        // Update min/max
        if (loadDuration < minLoadTime) minLoadTime = loadDuration
        if (loadDuration > maxLoadTime) maxLoadTime = loadDuration

        // Calculate averages
        val avgLoadTime = recentLoadTimes.average().roundToInt()
        val avgTotalLatency = recentTotalLatencies.average().roundToInt()

        // Get last 10 load times for display
        val last10 = recentLoadTimes.takeLast(10)

        // Update StateFlow
        _performanceMetrics.value = PerformanceMetrics(
            avgLoadTimeMs = avgLoadTime,
            minLoadTimeMs = minLoadTime,
            maxLoadTimeMs = maxLoadTime,
            avgTotalLatencyMs = avgTotalLatency,
            last10LoadTimes = last10,
            sampleCount = recentLoadTimes.size
        )

        // Log every 10 updates
        if (recentLoadTimes.size % 10 == 0) {
            println("⏱️ Performance (${recentLoadTimes.size} samples): " +
                    "avg=${avgLoadTime}ms, min=${minLoadTime}ms, max=${maxLoadTime}ms, " +
                    "latency=${avgTotalLatency}ms")
        }
    }

    fun onCleared() {
        // Stop background updates
        stressTestHandle?.stop()

        // Clear the observable collection
        // The collection and its observers will be cleaned up when garbage collected
        observableCollection = null

        // Cancel coroutine scope
        viewModelScope.cancel()
    }
}
