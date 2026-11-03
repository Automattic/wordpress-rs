package rs.wordpress.example.shared.ui.reader

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URI
import java.net.URISyntaxException

actual suspend fun readerHttpGet(url: String): ReaderHttpResponse = withContext(Dispatchers.IO) {
    try {
        val connection = URI(url).toURL().openConnection() as HttpURLConnection
        try {
            val status = connection.responseCode
            val stream = if (status in 200..299) connection.inputStream else connection.errorStream
            ReaderHttpResponse(status, stream?.bufferedReader()?.use { it.readText() }.orEmpty())
        } finally {
            connection.disconnect()
        }
    } catch (exception: URISyntaxException) {
        throw ReaderApiException("$url isn't a valid address", exception)
    } catch (exception: IOException) {
        throw ReaderApiException("Couldn't reach the Reader API: ${exception.message}", exception)
    }
}
