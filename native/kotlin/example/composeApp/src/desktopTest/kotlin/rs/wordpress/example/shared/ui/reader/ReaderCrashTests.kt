package rs.wordpress.example.shared.ui.reader

import androidx.compose.ui.platform.UriHandler
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

/** What the Reader screens only catch as a [ReaderApiException] stops being a crash. */
class ReaderCrashTests {
    private fun json(text: String) = Json.parseToJsonElement(text).jsonObject

    @Test
    fun aSiteThatCantBeInAnAddressIsAnApiError() = runTest {
        assertFailsWith<ReaderApiException> {
            readerHttpGet("https://public-api.wordpress.com/rest/v1.2/read/sites/foo bar/posts")
        }
    }

    @Test
    fun aResponseWithoutPostsIsAnApiError() {
        assertFailsWith<ReaderApiException> { readerPosts(json("""{"found": 0}""")) }
    }

    @Test
    fun postsThatArentAListAreAnApiError() {
        assertFailsWith<ReaderApiException> { readerPosts(json("""{"posts": 1}""")) }
    }

    @Test
    fun aPostThatIsntAnObjectIsAnApiError() {
        assertFailsWith<ReaderApiException> { readerPosts(json("""{"posts": [1]}""")) }
    }

    @Test
    fun aFieldThatIsntAStringIsEmpty() {
        assertEquals("", json("""{"title": {"rendered": "A"}}""").string("title"))
    }

    @Test
    fun anAddressNoAppCanOpenIsAnError() {
        val handler = object : UriHandler {
            override fun openUri(uri: String) = throw IllegalArgumentException("Can't open $uri.")
        }
        assertEquals("Can't open tel:+15550100.", handler.open("tel:+15550100"))
    }

    @Test
    fun anAddressThatOpensIsNotAnError() {
        val handler = object : UriHandler {
            override fun openUri(uri: String) = Unit
        }
        assertNull(handler.open("https://example.com"))
    }
}
