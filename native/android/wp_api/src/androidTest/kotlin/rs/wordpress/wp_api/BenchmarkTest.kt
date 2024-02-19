package rs.wordpress.wp_api

import android.util.Log
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
        val request: WpNetworkRequest
        val response: WpNetworkResponse
        val postListResponse: PostListResponse

        val timeToBuildRequest: Long
        val timeToMakeNetworkRequest: Long
        val timeToParseResponse: Long

        val totalTime = measureTimeMillis {
            timeToBuildRequest = measureTimeMillis {
                request = library.postListRequest()
            }
            timeToMakeNetworkRequest = measureTimeMillis {
                response = library.request(request)
            }
            timeToParseResponse = measureTimeMillis {
                postListResponse = parsePostListResponse(response)
            }
        }
        val firstPost = postListResponse.postList!!.first()
        assert(firstPost.title?.raw == "Hello world from Rust!")

        val jsonSize = jsonString(response).utf8Size()
        logBenchmarkResults("cross-platform", timeToBuildRequest, timeToMakeNetworkRequest, timeToParseResponse, totalTime, jsonSize)
    }

    @Test
    fun testKotlinBasedImplementation() {
        val request: WpNetworkRequest
        val response: WpNetworkResponse
        val postListResponse: List<KotlinPostObject>

        val timeToBuildRequest: Long
        val timeToMakeNetworkRequest: Long
        val timeToParseResponse: Long

        val totalTime = measureTimeMillis {
            timeToBuildRequest = measureTimeMillis {
                val url = siteUrl.plus("/wp-json/wp/v2/posts?context=edit");
                val headerMap = mapOf("Authorization" to "Basic {}".plus(authentication.authToken))
                request = WpNetworkRequest(RequestMethod.GET, url, headerMap)
            }
            timeToMakeNetworkRequest = measureTimeMillis {
                response = library.request(request)
            }
            timeToParseResponse = measureTimeMillis {
                postListResponse = kotlinParsePostListResponse(response)
            }
        }

        val firstPost: KotlinPostObject = postListResponse.first()
        assert(firstPost.title?.raw == "Hello world from Rust!")

        val jsonSize = jsonString(response).utf8Size()
        logBenchmarkResults("Kotlin", timeToBuildRequest, timeToMakeNetworkRequest, timeToParseResponse, totalTime, jsonSize)
    }

    private fun kotlinParsePostListResponse(response: WpNetworkResponse): List<KotlinPostObject> {
        return json.decodeFromString<List<KotlinPostObject>>(jsonString(response))
    }

    private fun logBenchmarkResults(implementationType: String, timeToBuildRequest: Long, timeToMakeNetworkRequest: Long, timeToParseResponse: Long, totalTime: Long, jsonSize: Long) {
        Log.println(Log.INFO, TAG_BENCHMARK,
            """
                Benchmark for $implementationType implementation:
                ---
                Time to build the request: $timeToBuildRequest
                Time to parse the response for json with utf8 size($jsonSize): $timeToParseResponse
                Time to build the request and parse the response: ${timeToBuildRequest + timeToParseResponse}
                
                [EXTRA] Time to make the request: $timeToMakeNetworkRequest
                [EXTRA] Total time: $totalTime
                
            """)
    }

    private fun jsonString(response: WpNetworkResponse): String = response.body.toString(Charsets.UTF_8)

    @Serializable
    data class KotlinPostObject(
        val id: UInt?,
        val title: KotlinPostTitle?,
        val content: KotlinPostContent?,
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
    )
}
