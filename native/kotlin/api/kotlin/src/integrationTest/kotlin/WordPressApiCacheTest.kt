import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.parallel.Execution
import org.junit.jupiter.api.parallel.ExecutionMode
import rs.wordpress.cache.kotlin.WordPressApiCache
import kotlin.test.assertEquals

@Execution(ExecutionMode.CONCURRENT)
class WordPressApiCacheTest {

    @Test
    fun testThatMigrationsWork() = runTest {
        assertEquals(8, WordPressApiCache().performMigrations())
    }
}
