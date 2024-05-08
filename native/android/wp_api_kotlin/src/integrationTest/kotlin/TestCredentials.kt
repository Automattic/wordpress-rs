package rs.wordpress.wp_api_kotlin

import java.io.File

data class TestCredentials(
    val siteUrl: String,
    val adminUsername: String,
    val adminPassword: String,
    val subscriberUsername: String,
    val subscriberPassword: String
) {
    companion object {
        fun get(): TestCredentials {
            val lineList = mutableListOf<String>()
            File(Companion::class.java.classLoader.getResource("test_credentials")!!.file).useLines { lines ->
                lines.forEach {
                    lineList.add(it)
                }
            }
            return TestCredentials(
                siteUrl = lineList[0],
                adminUsername = lineList[1],
                adminPassword = lineList[2],
                subscriberUsername = lineList[3],
                subscriberPassword = lineList[4],
            )
        }
    }
}
