import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import org.junit.jupiter.api.io.TempDir
import uniffi.wp_api.PostFormat
import uniffi.wp_mobile.LocalContentService
import uniffi.wp_mobile_local_content.FeaturedImage
import uniffi.wp_mobile_local_content.LocalContentException
import uniffi.wp_mobile_local_content.PageWorkingCopy
import uniffi.wp_mobile_local_content.PostWorkingCopy
import uniffi.wp_mobile_local_content.PublishDate
import uniffi.wp_mobile_local_content.RemotePostReference
import uniffi.wp_mobile_local_content.SiteIdentity
import uniffi.wp_mobile_local_content.StatusSetting
import uniffi.wp_mobile_local_content.Visibility
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
        status = StatusSetting.DRAFT,
        visibility = Visibility.Public,
        publishDate = PublishDate.Immediately,
        author = null,
        featuredImage = FeaturedImage.None
    )

    private fun post(title: String) = PostWorkingCopy(
        common = common(title),
        categories = emptyList(),
        tags = emptyList(),
        format = PostFormat.Standard,
        sticky = false
    )

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
                status = StatusSetting.PUBLISH,
                visibility = Visibility.PasswordProtected(password = "secret"),
                publishDate = PublishDate.At(date = Date(1_900_000_000_000L)),
                author = 7L,
                featuredImage = FeaturedImage.Remote(mediaId = 42L)
            ),
            categories = listOf(1L, 5L),
            tags = listOf("existing", "not on the site yet"),
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
}
