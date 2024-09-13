package rs.wordpress.api.kotlin

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import uniffi.wp_api.ParsedUrl
import java.io.File

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
    val subscriberPasswordUuid: String
) {
    companion object {
        val INSTANCE: TestCredentials by lazy(LazyThreadSafetyMode.SYNCHRONIZED) {
            val file = File(Companion::class.java.classLoader.getResource("test_credentials.json")!!.file)
            Json.decodeFromString<TestCredentials>(file.readText())
        }
    }

    val parsedSiteUrl by lazy { ParsedUrl.parse(siteUrl) }
}
