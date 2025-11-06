import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.parallel.Execution
import org.junit.jupiter.api.parallel.ExecutionMode
import rs.wordpress.api.kotlin.createSelfHostedService
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
        val service = createSelfHostedService()
        val postService = service.posts()

        // Setup: Insert a test post
        val postId = 42L
        postService.insertMockPostForTesting(postId, "Original Title")

        // Create observable entity and add observer
        val observableEntity = postService.getObservableEntityWithEditContext(postId)
        val callCount = AtomicInteger(0)

        observableEntity.addObserver {
            callCount.incrementAndGet()
        }

        // Observer should not have been called yet
        assertEquals(0, callCount.get())

        // Update the post - should trigger observer
        postService.updateMockPostForTesting(postId, "Updated Title")

        // Verify observer was called exactly once
        assertEquals(1, callCount.get())

        // Verify data was actually updated (using async version)
        val fullEntity = observableEntity.loadDataAsync()!!
        assertEquals("Updated Title", fullEntity.data.title.rendered)
    }

    @Test
    fun `observable entity supports multiple observers`() = runTest {
        val service = createSelfHostedService()
        val postService = service.posts()

        val postId = 100L
        postService.insertMockPostForTesting(postId, "Multi Observer Test")

        val observableEntity = postService.getObservableEntityWithEditContext(postId)

        val observer1Calls = AtomicInteger(0)
        val observer2Calls = AtomicInteger(0)

        observableEntity.addObserver { observer1Calls.incrementAndGet() }
        observableEntity.addObserver { observer2Calls.incrementAndGet() }

        // Update should trigger both observers
        postService.updateMockPostForTesting(postId, "Updated")

        assertEquals(1, observer1Calls.get())
        assertEquals(1, observer2Calls.get())
    }

    @Test
    fun `observable entity only fires for relevant updates`() = runTest {
        val service = createSelfHostedService()
        val postService = service.posts()

        // Create two different posts
        val post1Id = 200L
        val post2Id = 201L
        postService.insertMockPostForTesting(post1Id, "Post 1")
        postService.insertMockPostForTesting(post2Id, "Post 2")

        // Create observable entity for post1
        val observablePost1 = postService.getObservableEntityWithEditContext(post1Id)
        val post1Calls = AtomicInteger(0)
        observablePost1.addObserver { post1Calls.incrementAndGet() }

        // Update post2 - should NOT trigger post1's observer
        postService.updateMockPostForTesting(post2Id, "Post 2 Updated")
        assertEquals(0, post1Calls.get(), "Observer should not fire for unrelated post")

        // Update post1 - SHOULD trigger observer
        postService.updateMockPostForTesting(post1Id, "Post 1 Updated")
        assertEquals(1, post1Calls.get(), "Observer should fire for relevant post")
    }

    @Test
    fun `observers can be removed`() = runTest {
        val service = createSelfHostedService()
        val postService = service.posts()

        val postId = 300L
        postService.insertMockPostForTesting(postId, "Remove Test")

        val observableEntity = postService.getObservableEntityWithEditContext(postId)
        val callCount = AtomicInteger(0)

        val observer = { callCount.incrementAndGet(); Unit }
        observableEntity.addObserver(observer)

        // First update - should fire
        postService.updateMockPostForTesting(postId, "Update 1")
        assertEquals(1, callCount.get())

        // Remove observer
        observableEntity.removeObserver(observer)

        // Second update - should NOT fire
        postService.updateMockPostForTesting(postId, "Update 2")
        assertEquals(1, callCount.get(), "Observer should not fire after removal")
    }
}
