import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import org.junit.jupiter.api.io.TempDir
import rs.wordpress.api.kotlin.TestCredentials
import rs.wordpress.api.kotlin.assertSuccessAndRetrieveData
import rs.wordpress.api.kotlin.defaultApiClient
import uniffi.wp_api.AnyPostWithEditContext
import uniffi.wp_api.PostCommentStatus
import uniffi.wp_api.PostEndpointType
import uniffi.wp_api.PostFormat
import uniffi.wp_api.PostListParams
import uniffi.wp_api.PostRetrieveParams
import uniffi.wp_mobile.LocalContentService
import uniffi.wp_mobile_local_content.FeaturedImage
import uniffi.wp_mobile_local_content.LocalContentException
import uniffi.wp_mobile_local_content.PageWorkingCopy
import uniffi.wp_mobile_local_content.PostWorkingCopy
import uniffi.wp_mobile_local_content.PublishDate
import uniffi.wp_mobile_local_content.RemotePostConversionError
import uniffi.wp_mobile_local_content.RemotePostReference
import uniffi.wp_mobile_local_content.SiteIdentity
import uniffi.wp_mobile_local_content.StatusSetting
import uniffi.wp_mobile_local_content.Tag
import uniffi.wp_mobile_local_content.WorkingCopyCommon
import uniffi.wp_mobile_local_content.WorkingCopyTarget
import uniffi.wp_mobile_local_content.WpLocalContentStore
import java.nio.file.Path
import java.util.Date
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class LocalContentTest {
    @TempDir
    lateinit var tempDir: Path

    private val databasePath: String
        get() = tempDir.resolve("local_content.db").toString()

    private fun openService(): LocalContentService {
        val store = WpLocalContentStore(databasePath)
        store.performMigrations()
        return LocalContentService(store)
    }

    private fun common(title: String) = WorkingCopyCommon(
        title = title,
        content = "<!-- wp:paragraph --><p>$title</p><!-- /wp:paragraph -->",
        excerpt = "",
        slug = null,
        status = StatusSetting.Draft(password = null),
        publishDate = PublishDate.Immediately,
        author = null,
        featuredImage = FeaturedImage.None,
        commentStatus = null,
        pingStatus = null
    )

    private fun post(title: String) = PostWorkingCopy(
        common = common(title),
        categories = emptyList(),
        tags = emptyList(),
        format = PostFormat.Standard,
        sticky = false
    )

    private val client = defaultApiClient()
    private val site = SiteIdentity.SelfHosted(url = TestCredentials.INSTANCE.siteUrl)

    private suspend fun fetchFirst(endpoint: PostEndpointType): AnyPostWithEditContext {
        val first = client.request { requestBuilder ->
            requestBuilder.posts().listWithEditContext(endpoint, PostListParams())
        }.assertSuccessAndRetrieveData().data.first()
        return client.request { requestBuilder ->
            requestBuilder.posts().retrieveWithEditContext(endpoint, first.id, PostRetrieveParams())
        }.assertSuccessAndRetrieveData().data
    }

    private val newPostTarget = WorkingCopyTarget(
        site = SiteIdentity.SelfHosted(url = "https://example.com"),
        remotePost = null
    )

    @Test
    fun `opening an in-memory database throws`() {
        assertThrows<LocalContentException.InvalidDatabasePath> {
            WpLocalContentStore(":memory:")
        }
    }

    @Test
    fun `saved post working copy survives reopening the database`() {
        val target = WorkingCopyTarget(
            site = SiteIdentity.WordPressCom(siteId = 123_456_789uL),
            remotePost = RemotePostReference(postId = 99L, baseModifiedGmt = Date(1_700_000_000_000L))
        )
        val edited = PostWorkingCopy(
            common = common("Edited").copy(
                status = StatusSetting.Custom(status = "in-review", password = "secret"),
                publishDate = PublishDate.At(date = Date(1_900_000_000_000L)),
                author = 7L,
                featuredImage = FeaturedImage.Remote(mediaId = 42L),
                commentStatus = PostCommentStatus.Closed
            ),
            categories = listOf(1L, 5L),
            tags = listOf(Tag.Existing(id = 3L), Tag.New(name = "not on the site yet")),
            format = PostFormat.Aside,
            sticky = true
        )

        val service = openService()
        val id = service.createPostWorkingCopy(post("Original"), target)
        service.savePostWorkingCopy(id, edited)
        val read = assertNotNull(openService().readPostWorkingCopy(id))

        assertEquals(edited, read.workingCopy)
        assertEquals(target, read.target)
    }

    @Test
    fun `saved page working copy survives reopening the database`() {
        val edited = PageWorkingCopy(common = common("Edited page"), parent = 3L)

        val service = openService()
        val id = service.createPageWorkingCopy(
            PageWorkingCopy(common = common("Original page"), parent = null),
            newPostTarget
        )
        service.savePageWorkingCopy(id, edited)
        val read = assertNotNull(openService().readPageWorkingCopy(id))

        assertEquals(edited, read.workingCopy)
        assertEquals(newPostTarget, read.target)
    }

    @Test
    fun `saving a page as a post throws`() {
        val service = openService()
        val pageId = service.createPageWorkingCopy(
            PageWorkingCopy(common = common("A page"), parent = null),
            newPostTarget
        )

        assertThrows<LocalContentException.WorkingCopyNotFound> {
            service.savePostWorkingCopy(pageId, post("Title"))
        }
    }

    @Test
    fun `working copy from a fetched post points at it`() = runTest {
        val post = fetchFirst(PostEndpointType.Posts)
        val service = openService()

        val id = service.createPostWorkingCopyFromRemote(site, post)
        val read = assertNotNull(service.readPostWorkingCopy(id))

        assertEquals(post.title?.raw, read.workingCopy.common.title)
        assertEquals(post.content.raw, read.workingCopy.common.content)
        assertEquals(
            RemotePostReference(postId = post.id, baseModifiedGmt = post.modifiedGmt),
            read.target.remotePost
        )
    }

    @Test
    fun `working copy from a page fetched as a post throws`() = runTest {
        val page = fetchFirst(PostEndpointType.Pages)
        val service = openService()

        val error = assertThrows<LocalContentException.RemotePostConversion> {
            service.createPostWorkingCopyFromRemote(site, page)
        }
        assertEquals(RemotePostConversionError.WrongPostType(expected = "post", actual = "page"), error.error)
    }
}
