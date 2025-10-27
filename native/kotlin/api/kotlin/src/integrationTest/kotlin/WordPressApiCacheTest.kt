import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.parallel.Execution
import org.junit.jupiter.api.parallel.ExecutionMode
import rs.wordpress.cache.kotlin.WordPressApiCache
import rs.wordpress.cache.kotlin.WordPressApiCacheDelegate
import kotlin.test.assertEquals

@Execution(ExecutionMode.CONCURRENT)
class WordPressApiCacheTest {

    @Test
    fun testThatMigrationsWork() = runTest {
        assertEquals(3, WordPressApiCache().performMigrations())
    }

    @Test
    fun testBackgroundUpdateNotificationsWork() = runTest {
        var updateCount = 0
        val delegate = WordPressApiCacheDelegate(
            callback = { updateHook ->
                updateCount += 1
            }
        )

        val cache = WordPressApiCache(delegate = delegate)
        cache.startListeningForUpdates()

        val migrationCount = cache.performMigrations()
        assertEquals(updateCount, migrationCount)
    }
}
