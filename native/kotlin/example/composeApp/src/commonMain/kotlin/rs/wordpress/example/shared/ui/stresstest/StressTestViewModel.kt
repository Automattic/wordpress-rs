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

    private var stressTestHandle: StressTestHandle? = null
    private var observableCollection: ObservableCollection<*>? = null

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
        fun reloadPosts() {
            try {
                val allPosts = collection.loadData()
                println("Loaded ${allPosts.size} posts from collection")

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
            } catch (e: Exception) {
                println("ERROR loading posts: ${e.message}")
                e.printStackTrace()
            }
        }

        // Load initial data
        reloadPosts()

        // Add observer to reload all posts when any change occurs
        collection.addObserver {
            viewModelScope.launch(Dispatchers.IO) {
                println("Collection observer called!")
                reloadPosts()

                // Increment total updates counter (number of times observer fired)
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
