package rs.wordpress.wp_api

import android.util.Log
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import okio.utf8Size
import org.junit.Before
import org.junit.Test
import uniffi.wp_api.PostListResponse
import uniffi.wp_api.RequestMethod
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.parsePostListResponse
import kotlin.system.measureTimeMillis

private const val TAG_BENCHMARK = "rs.wordpress.wp_api.BENCHMARK"
private const val NUMBER_OF_ITERATIONS = 100

class BenchmarkTest {
    private val json = Json { ignoreUnknownKeys = true }
    private val siteUrl = "{omitted}"
    private val authentication = WpAuthentication(authToken = "{omitted}")
    private val library = Library(siteUrl, authentication)

    @Before
    fun setup() {
    }

    @Test
    fun testRustBasedImplementation() {
        var timeToBuildRequest: Long = 0
        var timeToMakeNetworkRequest: Long = 0
        var timeToParseResponse: Long = 0

        var totalTime: Long = 0
        var jsonSize: Long = 0

        repeat(NUMBER_OF_ITERATIONS) {
            val request: WpNetworkRequest
            val response: WpNetworkResponse
            val postListResponse: PostListResponse

            totalTime += measureTimeMillis {
                timeToBuildRequest += measureTimeMillis {
                    request = library.postListRequest()
                }
                timeToMakeNetworkRequest += measureTimeMillis {
                    response = library.request(request)
                }
                timeToParseResponse += measureTimeMillis {
                    postListResponse = parsePostListResponse(response)
                }
            }
            val firstPost = postListResponse.postList!!.first()
            assert(firstPost.title?.raw == "Hello world from Rust!")

            jsonSize += jsonString(response).utf8Size()
        }
        logBenchmarkResults(
            "cross-platform",
            timeToBuildRequest,
            timeToMakeNetworkRequest,
            timeToParseResponse,
            totalTime,
            jsonSize
        )
    }

    @Test
    fun testKotlinBasedImplementation() {
        var timeToBuildRequest: Long = 0
        var timeToMakeNetworkRequest: Long = 0
        var timeToParseResponse: Long = 0

        var totalTime: Long = 0
        var jsonSize: Long = 0

        repeat(NUMBER_OF_ITERATIONS) {
            val request: WpNetworkRequest
            val response: WpNetworkResponse
            val postListResponse: List<KotlinPostObject>

            totalTime += measureTimeMillis {

                timeToBuildRequest += measureTimeMillis {
                    val url = siteUrl.plus("/wp-json/wp/v2/posts?context=edit");
                    val headerMap =
                        mapOf("Authorization" to "Basic {}".plus(authentication.authToken))
                    request = WpNetworkRequest(RequestMethod.GET, url, headerMap)
                }
                timeToMakeNetworkRequest += measureTimeMillis {
                    response = library.request(request)
                }
                timeToParseResponse += measureTimeMillis {
                    postListResponse = kotlinParsePostListResponse(response)
                }
            }

            val firstPost: KotlinPostObject = postListResponse.first()
            assert(firstPost.title?.raw == "Hello world from Rust!")

            jsonSize += jsonString(response).utf8Size()
        }
        logBenchmarkResults(
            "Kotlin",
            timeToBuildRequest,
            timeToMakeNetworkRequest,
            timeToParseResponse,
            totalTime,
            jsonSize
        )
    }

    private fun kotlinParsePostListResponse(response: WpNetworkResponse): List<KotlinPostObject> {
        return json.decodeFromString<List<KotlinPostObject>>(jsonString(response))
    }

    private fun logBenchmarkResults(
        implementationType: String,
        timeToBuildRequest: Long,
        timeToMakeNetworkRequest: Long,
        timeToParseResponse: Long,
        totalTime: Long,
        jsonSize: Long
    ) {
        Log.println(
            Log.INFO, TAG_BENCHMARK,
            """
                Benchmark for $implementationType implementation for an average of $NUMBER_OF_ITERATIONS iterations:
                ---
                Average time to build the request: ${timeToBuildRequest / NUMBER_OF_ITERATIONS}
                Average time to parse the response for json with utf8 size(${jsonSize / NUMBER_OF_ITERATIONS}): ${timeToParseResponse / NUMBER_OF_ITERATIONS}
                Average time to build the request and parse the response: ${(timeToBuildRequest + timeToParseResponse) / NUMBER_OF_ITERATIONS}
                
                [EXTRA] Average time to make the request: ${timeToMakeNetworkRequest / NUMBER_OF_ITERATIONS}
                [EXTRA] Average total time: ${totalTime / NUMBER_OF_ITERATIONS}
            """.trimIndent()
        )
    }

    private fun jsonString(response: WpNetworkResponse): String =
        response.body.toString(Charsets.UTF_8)

    @Serializable
    data class KotlinPostObject(
        val id: UInt?,
        val date: String?,
        @SerialName("date_gmt")
        val dateGmt: String?,
        val guid: KotlinPostGuid?,
        val modified: String?,
        @SerialName("modified_gmt")
        val modifiedGmt: String?,
        val password: String?,
        val slug: String?,
        val status: String?,
        val link: String?,
        val title: KotlinPostTitle?,
        val content: KotlinPostContent?,
        val excerpt: KotlinPostExcerpt?,
        val author: UInt?,
        @SerialName("featured_media")
        val featuredMedia: UInt?,
        @SerialName("comment_status")
        val commentStatus: String?,
        @SerialName("ping_status")
        val pingStatus: String?,
        val sticky: Boolean?,
        val template: String?,
        val format: String?,
        val meta: KotlinPostMeta?,
        val categories: List<UInt>?,
        val tags: List<UInt>?,
    )

    @Serializable
    data class KotlinPostGuid(
        val raw: String?,
        val rendered: String?,
    )

    @Serializable
    data class KotlinPostTitle(
        val raw: String?,
        val rendered: String?,
    )

    @Serializable
    data class KotlinPostContent(
        val raw: String?,
        val rendered: String?,
        val protected: Boolean?,
        @SerialName("block_version")
        val blockVersion: UInt?,
    )

    @Serializable
    data class KotlinPostExcerpt(
        val raw: String?,
        val rendered: String?,
        val protected: Boolean?,
    )

    @Serializable
    data class KotlinPostMeta(
        val footnotes: String?,
    )
}
