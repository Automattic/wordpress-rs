package rs.wordpress.api.kotlin

import java.io.File

class DefaultFileResolver : FileResolver {
    override fun getFile(path: String): File? = File(path)
}
