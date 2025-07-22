package rs.wordpress.api.kotlin

import java.io.File

interface FileResolver {
    fun getFile(path: String): File?
}
