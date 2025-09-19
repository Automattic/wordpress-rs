package rs.wordpress.api.cache.kotlin

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.parallel.Execution
import org.junit.jupiter.api.parallel.ExecutionMode
import rs.wordpress.cache.kotlin.WordPressApiCache
import kotlin.test.assertEquals

@Execution(ExecutionMode.CONCURRENT)
class WordPressApiCacheTest {

    @Test
    fun testThatMigrationsWork() {
        assertEquals(2, WordPressApiCache().performMigrations())
    }
}
