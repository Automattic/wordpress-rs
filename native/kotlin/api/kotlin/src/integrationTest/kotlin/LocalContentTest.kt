import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import org.junit.jupiter.api.io.TempDir
import uniffi.wp_mobile.LocalContentService
import uniffi.wp_mobile_local_content.LocalContentException
import uniffi.wp_mobile_local_content.PostWorkingCopy
import uniffi.wp_mobile_local_content.WpLocalContentStore
import java.nio.file.Path
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class LocalContentTest {
    @TempDir
    lateinit var tempDir: Path

    private val databasePath: String
        get() = tempDir.resolve("local_content.db").toString()

    private fun openStore(): WpLocalContentStore {
        val store = WpLocalContentStore(databasePath)
        store.performMigrations()
        return store
    }

    @Test
    fun `opening an in-memory database throws`() {
        assertThrows<LocalContentException.InvalidDatabasePath> {
            WpLocalContentStore(":memory:")
        }
    }

    @Test
    fun `create then read returns the created post working copy`() {
        val service = LocalContentService(openStore())
        val created = PostWorkingCopy(title = "Draft title", content = "Draft content")

        val id = service.createPostWorkingCopy(created)
        val read = assertNotNull(service.readPostWorkingCopy(id))

        assertEquals(id, read.id)
        assertEquals(created, read.workingCopy)
    }

    @Test
    fun `save then read returns the saved post working copy`() {
        val service = LocalContentService(openStore())
        val id = service.createPostWorkingCopy(PostWorkingCopy(title = "First", content = "first"))

        val saved = PostWorkingCopy(title = "Second", content = "second")
        service.savePostWorkingCopy(id, saved)
        val read = assertNotNull(service.readPostWorkingCopy(id))

        assertEquals(saved, read.workingCopy)
    }

    @Test
    fun `saving an unknown post working copy throws`() {
        val service = LocalContentService(openStore())
        val unknownId = "00000000-0000-4000-8000-000000000000"

        assertThrows<LocalContentException.PostWorkingCopyNotFound> {
            service.savePostWorkingCopy(unknownId, PostWorkingCopy(title = "Title", content = "content"))
        }
    }

    @Test
    fun `post working copy survives reopening the database`() {
        val created = PostWorkingCopy(title = "Kept", content = "kept content")

        val id = LocalContentService(openStore()).createPostWorkingCopy(created)
        val read = assertNotNull(LocalContentService(openStore()).readPostWorkingCopy(id))

        assertEquals(created, read.workingCopy)
    }
}
