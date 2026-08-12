package rs.wordpress.api.kotlin

import okio.Buffer
import okio.Source
import okio.Timeout
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import java.io.File
import java.io.IOException

class MediaFileRequestBodyTest {

    @Test
    fun readFailureIsTaggedWithFilePath() {
        // A read failure (the file, not the socket) is reported as `MediaFileUnreadableException`
        // carrying the path, so the executor can surface `MediaFileUnreadable` naming the file.
        val failing = object : Source {
            override fun read(sink: Buffer, byteCount: Long): Long = throw IOException("disk gone")
            override fun timeout(): Timeout = Timeout.NONE
            override fun close() = Unit
        }

        val error = assertThrows(MediaFileUnreadableException::class.java) {
            readTaggingSource(FILE_PATH, failing).read(Buffer(), READ_SIZE)
        }

        assertEquals(FILE_PATH, error.filePath)
    }

    @Test
    fun successfulReadIsNotTagged() {
        val out = Buffer()

        readTaggingSource(FILE_PATH, Buffer().writeUtf8(CONTENT)).read(out, READ_SIZE)

        assertEquals(CONTENT, out.readUtf8())
    }

    @Test
    fun writeToStreamsTheFileContents() {
        val file = File.createTempFile("upload", ".bin")
        try {
            file.writeText(CONTENT)
            val sink = Buffer()

            MediaFileRequestBody(file, file.path, null).writeTo(sink)

            assertEquals(CONTENT, sink.readUtf8())
        } finally {
            file.delete()
        }
    }

    @Test
    fun writeToTagsAFileThatCannotBeOpened() {
        // The file passed the pre-upload check but is gone by the time OkHttp reads it: opening
        // its source fails, and that surfaces as `MediaFileUnreadable`, not a generic error.
        val body = MediaFileRequestBody(File(MISSING_PATH), MISSING_PATH, null)

        val error = assertThrows(MediaFileUnreadableException::class.java) {
            body.writeTo(Buffer())
        }

        assertEquals(MISSING_PATH, error.filePath)
    }

    private companion object {
        const val FILE_PATH = "/tmp/uploads/photo.jpg"
        const val MISSING_PATH = "/nonexistent/directory/photo.jpg"
        const val CONTENT = "hello"
        const val READ_SIZE = 4096L
    }
}
