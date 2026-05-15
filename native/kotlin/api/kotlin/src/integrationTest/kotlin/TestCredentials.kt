package rs.wordpress.api.kotlin

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.File
import java.net.URI
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
    @SerialName("author_username")
    val authorUsername: String,
    @SerialName("author_password")
    val authorPassword: String,
    @SerialName("password_protected_post_id")
    val passwordProtectedPostId: Long,
    @SerialName("password_protected_post_password")
    val passwordProtectedPostPassword: String,
    @SerialName("password_protected_post_title")
    val passwordProtectedPostTitle: String,
    @SerialName("password_protected_comment_id")
    val passwordProtectedCommentId: Long,
    @SerialName("password_protected_comment_author")
    val passwordProtectedCommentAuthor: String,
    @SerialName("trashed_post_id")
    val trashedPostId: Long,
    @SerialName("first_post_date_gmt")
    val firstPostDateGmt: String,
    @SerialName("wordpress_core_version")
    val wordpressCoreVersion: String,
    @SerialName("integration_test_custom_template_id")
    val integrationTestCustomTemplateId: String,
    @SerialName("autosaved_template_id")
    val autosavedTemplateId: String,
    @SerialName("autosave_id_for_autosaved_template")
    val autosaveIdForAutosavedTemplate: Long,
    @SerialName("integration_test_custom_template_part_id")
    val integrationTestCustomTemplatePartId: String,
    @SerialName("autosaved_template_part_id")
    val autosavedTemplatePartId: String,
    @SerialName("autosave_id_for_autosaved_template_part")
    val autosaveIdForAutosavedTemplatePart: Long,
    @SerialName("revision_id_for_custom_template_part")
    val revisionIdForCustomTemplatePart: Long,
    @SerialName("revision_id_for_custom_template")
    val revisionIdForCustomTemplate: Long,
    @SerialName("revisioned_post_id")
    val revisionedPostId: Long,
    @SerialName("primary_menu_location")
    val primaryMenuLocation: String,
    @SerialName("nav_menu_item_id")
    val navMenuItemId: Long,
    @SerialName("block_id")
    val blockId: Long,
    @SerialName("global_styles_id")
    val globalStylesId: Long,
    @SerialName("revision_id_for_block_id")
    val revisionIdForBlockId: Long,
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

    val apiRootUrl by lazy { URI("$siteUrl/wp-json").toURL() }
}
