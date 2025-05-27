package rs.wordpress.api.kotlin

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.File
import java.net.URL
import java.text.SimpleDateFormat
import java.util.TimeZone

@Serializable
data class TestCredentials(
    @SerialName("site_url")
    val siteUrl: String,
    @SerialName("admin_username")
    val adminUsername: String,
    @SerialName("admin_password")
    val adminPassword: String,
    @SerialName("admin_password_uuid")
    val adminPasswordUuid: String,
    @SerialName("subscriber_username")
    val subscriberUsername: String,
    @SerialName("subscriber_password")
    val subscriberPassword: String,
    @SerialName("subscriber_password_uuid")
    val subscriberPasswordUuid: String,
    @SerialName("first_post_date_gmt")
    val firstPostDateGmt: String
) {
    companion object {
        private val json by lazy {
            Json { ignoreUnknownKeys = true }
        }
        val INSTANCE: TestCredentials by lazy(LazyThreadSafetyMode.SYNCHRONIZED) {
            val file =
                File(Companion::class.java.classLoader.getResource("test_credentials.json")!!.file)
            json.decodeFromString<TestCredentials>(file.readText())
        }

        val UTC_DATE_FORMAT by lazy {
            SimpleDateFormat("yyyy-MM-dd HH:mm:ss").apply {
                this.timeZone = TimeZone.getTimeZone("UTC")
            }
        }
    }

    val apiRootUrl by lazy { URL("$siteUrl/wp-json") }
}
