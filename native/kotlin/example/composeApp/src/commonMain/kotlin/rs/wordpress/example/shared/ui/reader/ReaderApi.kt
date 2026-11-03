package rs.wordpress.example.shared.ui.reader

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.longOrNull
import uniffi.wp_mobile.ReaderPostContent

/** An HTTP response's status and body. */
class ReaderHttpResponse(val status: Int, val body: String)

/** Fetches `url` with a GET request. */
expect suspend fun readerHttpGet(url: String): ReaderHttpResponse

class ReaderApiException(message: String, cause: Throwable? = null) : Exception(message, cause)

/**
 * The public Reader API, which the Reader shows posts from. Public sites' posts don't need signing in.
 */
object ReaderApi {
    private const val ROOT = "https://public-api.wordpress.com/rest/v1.2/read/sites"

    data class PostSummary(val id: ULong, val title: String, val date: String)

    data class Post(val title: String, val url: String, val content: String, val siteIsPrivate: Boolean)

    suspend fun posts(site: String): List<PostSummary> {
        val json = get("$ROOT/$site/posts?number=30&fields=ID,title,date")
        return readerPosts(json).map { fields ->
            PostSummary(
                id = fields.long("ID").toULong(),
                title = plainText(fields.string("title")),
                date = fields.string("date").take(10)
            )
        }
    }

    suspend fun post(site: String, postId: ULong): Post {
        val json = get("$ROOT/$site/posts/$postId")
        return Post(
            title = plainText(json.string("title")),
            url = json.string("URL"),
            content = json.string("content"),
            siteIsPrivate = json.primitive("site_is_private")?.booleanOrNull ?: false
        )
    }

    private suspend fun get(url: String): JsonObject {
        val response = readerHttpGet(url)
        val json = runCatching { Json.parseToJsonElement(response.body).jsonObject }.getOrElse {
            throw ReaderApiException("The Reader API sent something other than JSON (${response.status})", it)
        }
        if (response.status !in 200..299) {
            val message = json.primitive("message")?.contentOrNull ?: "no message"
            throw ReaderApiException("The Reader API answered ${response.status}: $message")
        }
        return json
    }

    private fun JsonObject.long(name: String): Long =
        primitive(name)?.longOrNull ?: throw ReaderApiException("A post has no $name")

    /** The text of the API's HTML, like a title with `&#8217;` in it. */
    private fun plainText(html: String): String = ReaderPostContent(html, null).use { it.plainText() }
}

/** The posts in a response from the Reader API. */
internal fun readerPosts(json: JsonObject): List<JsonObject> {
    val posts = json["posts"] as? JsonArray ?: throw ReaderApiException("The response has no list of posts")
    return posts.map { it as? JsonObject ?: throw ReaderApiException("A post isn't an object") }
}

private fun JsonObject.primitive(name: String): JsonPrimitive? = this[name] as? JsonPrimitive

internal fun JsonObject.string(name: String): String = primitive(name)?.contentOrNull.orEmpty()

/** The site the Reader API knows `input` by: a domain or a site ID, from an address or a domain. */
fun readerSite(input: String): String =
    input.trim().substringAfter("://").substringBefore('/').substringBefore('?')
