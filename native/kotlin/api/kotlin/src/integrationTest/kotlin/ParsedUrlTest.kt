package rs.wordpress.api.kotlin

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.parallel.Execution
import org.junit.jupiter.api.parallel.ExecutionMode
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.QueryPair
import kotlin.test.assertEquals

@Execution(ExecutionMode.CONCURRENT)
class ParsedUrlTest {

    @Test
    fun testAppendQueryPairsToPathRoot() {
        val url = ParsedUrl.parse("https://example.com/wp-json/wp/v2/themes")
        val result = url.byAppendingQueryPairs(
            listOf(
                QueryPair(name = "context", value = "edit"),
                QueryPair(name = "status", value = "active")
            )
        )
        assertEquals(
            "https://example.com/wp-json/wp/v2/themes?context=edit&status=active",
            result.url()
        )
    }

    @Test
    fun testAppendQueryPairsToRestRouteRoot() {
        val url = ParsedUrl.parse("https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes")
        val result = url.byAppendingQueryPairs(
            listOf(
                QueryPair(name = "context", value = "edit"),
                QueryPair(name = "status", value = "active")
            )
        )
        assertEquals(
            "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes&context=edit&status=active",
            result.url()
        )
    }

    @Test
    fun testAppendQueryPairsFormUrlencodesReservedCharacters() {
        val url = ParsedUrl.parse("https://example.com/wp-json/wp/v2/themes")
        val result = url.byAppendingQueryPairs(
            listOf(QueryPair(name = "exclude", value = "core,gutenberg"))
        )
        assertEquals(
            "https://example.com/wp-json/wp/v2/themes?exclude=core%2Cgutenberg",
            result.url()
        )
    }

    @Test
    fun testAppendEmptyQueryPairsLeavesUrlUnchanged() {
        val url = ParsedUrl.parse("https://example.com/wp-json/wp/v2/themes")
        val result = url.byAppendingQueryPairs(emptyList())
        assertEquals(url.url(), result.url())
    }
}
