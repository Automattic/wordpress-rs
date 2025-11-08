package rs.wordpress.example.shared.ui.stresstest

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import rs.wordpress.cache.kotlin.ObservableEntity
import rs.wordpress.cache.kotlin.WordPressApiCache
import rs.wordpress.cache.kotlin.getObservableEntityWithEditContext
import uniffi.wp_mobile.MockPostService
import uniffi.wp_mobile.StressTestHandle
import uniffi.wp_mobile.WpSelfHostedService
import uniffi.wp_mobile_cache.EntityKey

class StressTestViewModel(
    private val mockPostService: MockPostService,
    private val selfHostedService: WpSelfHostedService,
    private val cache: WordPressApiCache
) {
    private val viewModelScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    private val _posts = MutableStateFlow<List<PostDisplayData>>(emptyList())
    val posts: StateFlow<List<PostDisplayData>> = _posts.asStateFlow()

    private val _totalUpdates = MutableStateFlow(0L)
    val totalUpdates: StateFlow<Long> = _totalUpdates.asStateFlow()

    private val _isRunning = MutableStateFlow(false)
    val isRunning: StateFlow<Boolean> = _isRunning.asStateFlow()

    private var stressTestHandle: StressTestHandle? = null
    private val observableEntities = mutableListOf<ObservableEntity<*>>()
    private val updateCounts = mutableMapOf<EntityKey, Int>()

    init {
        startStressTest()
    }

    private fun startStressTest() {
        println("Starting stress test...")
        // Generate 1000 posts
        val entityIds = mockPostService.generateAndInsertPosts(1000u)
        println("Generated ${entityIds.size} posts")

        val postService = selfHostedService.posts()

        // Create observable entities for all posts
        val postDataList = mutableListOf<PostDisplayData>()

        entityIds.forEach { entityId ->
            val observableEntity = postService.getObservableEntityWithEditContext(entityId)

            // Load initial data
            val fullEntity = observableEntity.loadData()
            if (fullEntity != null) {
                val entityKey = fullEntity.entityId.toKey()
                val postData = PostDisplayData(
                    entityId = fullEntity.entityId,
                    title = fullEntity.data.title.rendered,
                    contentPreview = fullEntity.data.content.rendered.take(100),
                    status = fullEntity.data.status.toString(),
                    author = fullEntity.data.author?.toString(),
                    date = fullEntity.data.date,
                    modified = fullEntity.data.modified,
                    updateCount = 0
                )
                postDataList.add(postData)
                updateCounts[entityKey] = 0
            }

            // Add observer to update UI when post changes
            observableEntity.addObserver {
                viewModelScope.launch(Dispatchers.IO) {
                    try {
                        println("Observer called!")
                        val updatedEntity = observableEntity.loadData()
                        if (updatedEntity != null) {
                            println("Updated entity loaded: ${updatedEntity.data.title.rendered}")
                            val entityKey = updatedEntity.entityId.toKey()
                            val currentCount = updateCounts[entityKey] ?: 0
                            updateCounts[entityKey] = currentCount + 1

                            val updatedPostData = PostDisplayData(
                                entityId = updatedEntity.entityId,
                                title = updatedEntity.data.title.rendered,
                                contentPreview = updatedEntity.data.content.rendered.take(100),
                                status = updatedEntity.data.status.toString(),
                                author = updatedEntity.data.author?.toString(),
                                date = updatedEntity.data.date,
                                modified = updatedEntity.data.modified,
                                updateCount = updateCounts[entityKey] ?: 0
                            )

                            // Switch to Main thread only for StateFlow updates
                            withContext(Dispatchers.Main) {
                                // Update the posts list
                                val currentPosts = _posts.value.toMutableList()
                                val index = currentPosts.indexOfFirst {
                                    it.entityId.toKey() == updatedEntity.entityId.toKey()
                                }
                                if (index != -1) {
                                    currentPosts[index] = updatedPostData
                                    _posts.value = currentPosts
                                }

                                // Update total count
                                _totalUpdates.value = updateCounts.values.sum().toLong()
                            }
                        } else {
                            println("WARNING: loadData() returned null for entity")
                        }
                    } catch (e: Exception) {
                        println("ERROR in observer: ${e.message}")
                        e.printStackTrace()
                    }
                }
            }

            observableEntities.add(observableEntity)
        }

        _posts.value = postDataList
        _isRunning.value = true

        println("Starting random updates for ${entityIds.size} posts...")
        // Start random updates with 50ms delay
        stressTestHandle = mockPostService.startRandomUpdates(entityIds, 0.05)
        println("Random updates started!")
    }

    fun onCleared() {
        // Stop background updates
        stressTestHandle?.stop()

        // Remove all observers
        observableEntities.forEach { entity ->
            // ObservableEntity doesn't expose removeAllObservers, so we'll just clear the list
            // The observers will be cleaned up when the entities are garbage collected
        }
        observableEntities.clear()

        // Cancel coroutine scope
        viewModelScope.cancel()
    }
}
