package rs.wordpress.api.kotlin

import java.io.File

data class TestCredentials(
    val siteUrl: String,
    val adminUsername: String,
    val adminPassword: String,
    val adminPasswordUuid: String,
    val subscriberUsername: String,
    val subscriberPassword: String,
    val subscriberPasswordUuid: String
) {
    companion object {
        val INSTANCE: TestCredentials by lazy(LazyThreadSafetyMode.SYNCHRONIZED) {
            val lineList = mutableListOf<String>()
            val file = File(Companion::class.java.classLoader.getResource("test_credentials")!!.file)
            file.useLines { lines ->
                lines.forEach {
                    lineList.add(it)
                }
            }
            TestCredentials(
                siteUrl = lineList[0],
                adminUsername = lineList[1],
                adminPassword = lineList[2],
                adminPasswordUuid = lineList[3],
                subscriberUsername = lineList[4],
                subscriberPassword = lineList[5],
                subscriberPasswordUuid = lineList[6],
            )
        }
    }
}
