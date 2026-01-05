import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.parallel.Execution
import org.junit.jupiter.api.parallel.ExecutionMode
import rs.wordpress.api.kotlin.createTestServiceContext
import rs.wordpress.cache.kotlin.getObservableEntityWithEditContext
import java.util.concurrent.atomic.AtomicInteger
import kotlin.test.assertEquals

/**
 * Tests for the Kotlin-specific ObservableEntity pattern.
 *
 * ObservableEntity is a platform-native wrapper that:
 * - Wraps Entity<T> from Rust
 * - Manages observers in Kotlin memory
 * - Automatically receives database change notifications
 * - Filters updates using Entity's is_relevant_update() method
 */
@Execution(ExecutionMode.CONCURRENT)
class ObservableEntityTest {

    @Test
    fun `observable entity notifies observers when database updates`() = runTest {
        val context = createTestServiceContext()
        val postService = context.service.posts()
        val mockPostService = context.mockPostService

        // Setup: Insert a test post
        val postId = 42L
        val entityId = mockPostService.insertMockPost(postId, "Original Title")

        // Create observable entity and add observer
        val observableEntity = postService.getObservableEntityWithEditContext(entityId)
        val callCount = AtomicInteger(0)

        observableEntity.addObserver {
            callCount.incrementAndGet()
        }

        // Observer should not have been called yet
        assertEquals(0, callCount.get())

        // Update the post - should trigger observer
        mockPostService.updateMockPost(postId, "Updated Title")

        // Verify observer was called exactly once
        assertEquals(1, callCount.get())

        // Verify data was actually updated
        val fullEntity = observableEntity.loadData()!!
        assertEquals("Updated Title", fullEntity.data.title.rendered)
    }

    @Test
    fun `observable entity supports multiple observers`() = runTest {
        val context = createTestServiceContext()
        val postService = context.service.posts()
        val mockPostService = context.mockPostService

        val postId = 100L
        val entityId = mockPostService.insertMockPost(postId, "Multi Observer Test")

        val observableEntity = postService.getObservableEntityWithEditContext(entityId)

        val observer1Calls = AtomicInteger(0)
        val observer2Calls = AtomicInteger(0)

        observableEntity.addObserver { observer1Calls.incrementAndGet() }
        observableEntity.addObserver { observer2Calls.incrementAndGet() }

        // Update should trigger both observers
        mockPostService.updateMockPost(postId, "Updated")

        assertEquals(1, observer1Calls.get())
        assertEquals(1, observer2Calls.get())
    }

    @Test
    fun `observable entity only fires for relevant updates`() = runTest {
        val context = createTestServiceContext()
        val postService = context.service.posts()
        val mockPostService = context.mockPostService

        // Create two different posts
        val post1Id = 200L
        val post2Id = 201L
        val entity1Id = mockPostService.insertMockPost(post1Id, "Post 1")
        mockPostService.insertMockPost(post2Id, "Post 2")

        // Create observable entity for post1
        val observablePost1 = postService.getObservableEntityWithEditContext(entity1Id)
        val post1Calls = AtomicInteger(0)
        observablePost1.addObserver { post1Calls.incrementAndGet() }

        // Update post2 - should NOT trigger post1's observer
        mockPostService.updateMockPost(post2Id, "Post 2 Updated")
        assertEquals(0, post1Calls.get(), "Observer should not fire for unrelated post")

        // Update post1 - SHOULD trigger observer
        mockPostService.updateMockPost(post1Id, "Post 1 Updated")
        assertEquals(1, post1Calls.get(), "Observer should fire for relevant post")
    }

    @Test
    fun `observers can be removed`() = runTest {
        val context = createTestServiceContext()
        val postService = context.service.posts()
        val mockPostService = context.mockPostService

        val postId = 300L
        val entityId = mockPostService.insertMockPost(postId, "Remove Test")

        val observableEntity = postService.getObservableEntityWithEditContext(entityId)
        val callCount = AtomicInteger(0)

        val observer = { callCount.incrementAndGet(); Unit }
        observableEntity.addObserver(observer)

        // First update - should fire
        mockPostService.updateMockPost(postId, "Update 1")
        assertEquals(1, callCount.get())

        // Remove observer
        observableEntity.removeObserver(observer)

        // Second update - should NOT fire
        mockPostService.updateMockPost(postId, "Update 2")
        assertEquals(1, callCount.get(), "Observer should not fire after removal")
    }

    @Test
    fun `bulk insert posts and verify count`() = runTest {
        val context = createTestServiceContext()
        val postService = context.service.posts()
        val mockPostService = context.mockPostService

        // Verify initial count is 0
        val initialCount = postService.countEditContext()
        assertEquals(0, initialCount)

        // Generate and insert 50 posts
        val postCount = 50u
        val entityIds = mockPostService.generateAndInsertPosts(postCount)

        // Verify the correct number of entity IDs were returned
        assertEquals(postCount.toInt(), entityIds.size)

        // Verify the count matches the number of posts inserted
        val finalCount = postService.countEditContext()
        assertEquals(postCount.toLong(), finalCount)
    }

    @Test
    fun `stress test with random updates triggers observers`() = runTest {
        val context = createTestServiceContext()
        val postService = context.service.posts()
        val mockPostService = context.mockPostService

        // Generate a small set of posts for stress testing
        val postCount = 5u
        val entityIds = mockPostService.generateAndInsertPosts(postCount)

        // Create observables for all posts and count notifications
        val observedCount = AtomicInteger(0)
        val observables = entityIds.map { entityId ->
            postService.getObservableEntityWithEditContext(entityId).apply {
                addObserver { observedCount.incrementAndGet() }
            }
        }

        // Start random updates with 50ms delay
        val handle = mockPostService.startRandomUpdates(entityIds, 0.05)

        // Let it run for ~500ms (should get roughly 10 updates)
        Thread.sleep(500)

        // Stop the updates
        handle.stop()

        // Get the final counts
        val updateCount = handle.updateCount()
        val totalObserved = observedCount.get()

        // Verify we got a reasonable number of updates
        // Update count should be > 0 and roughly around 10 (500ms / 50ms)
        assert(updateCount > 0u) { "Should have performed some updates" }
        assert(updateCount >= 8u) { "Should have performed at least 8 updates in 500ms with 50ms delay" }

        // Observed count should be close to update count
        // It might be slightly less due to timing, but should be reasonably close
        assert(totalObserved > 0) { "Should have observed some updates" }
        assert(totalObserved.toULong() >= updateCount - 2u) {
            "Observed count ($totalObserved) should be close to update count ($updateCount)"
        }

        println("Stress test completed: $updateCount updates, $totalObserved observed events")
    }
}
