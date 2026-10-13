import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import uniffi.wp_mobile.LocalContentService
import uniffi.wp_mobile_local_content.LocalContentException
import uniffi.wp_mobile_local_content.PostWorkingCopy
import uniffi.wp_mobile_local_content.WpLocalContentStore
import kotlin.io.path.createTempDirectory
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class LocalContentTest {
    private fun openStore(path: String? = null): WpLocalContentStore {
        val store = WpLocalContentStore(path)
        store.performMigrations()
        return store
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
        val path = createTempDirectory().resolve("local_content.db").toString()
        val created = PostWorkingCopy(title = "Kept", content = "kept content")

        val id = LocalContentService(openStore(path)).createPostWorkingCopy(created)
        val read = assertNotNull(LocalContentService(openStore(path)).readPostWorkingCopy(id))

        assertEquals(created, read.workingCopy)
    }
}
