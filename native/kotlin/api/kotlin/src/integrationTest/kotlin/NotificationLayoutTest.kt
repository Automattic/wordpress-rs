package rs.wordpress.api.kotlin

import org.junit.jupiter.api.Test
import uniffi.wp_api.NotificationActor
import uniffi.wp_api.NotificationBlock
import uniffi.wp_api.NotificationBlockKind
import uniffi.wp_api.NotificationBlockRole
import uniffi.wp_api.NotificationIndices
import uniffi.wp_api.NotificationLabel
import uniffi.wp_api.NotificationLayoutContent
import uniffi.wp_api.NotificationLayoutText
import uniffi.wp_api.NotificationMeta
import uniffi.wp_api.NotificationRange
import uniffi.wp_api.NotificationRangeKind
import uniffi.wp_api.NotificationType
import uniffi.wp_api.WpComNotification
import uniffi.wp_api.localizeNotificationLabel
import uniffi.wp_api.localizedDescription
import uniffi.wp_api.notificationLayout
import java.util.Locale
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class NotificationLayoutTest {
    // Twelve people liked a post: the note lists two of them, then a line about the rest.
    private val note = WpComNotification(
        id = 10_000_000_002uL,
        noteType = NotificationType.Like,
        title = "Likes",
        url = "https://garden.example.com/winter",
        body = listOf(
            NotificationBlock(text = "Sam", kind = NotificationBlockKind.User),
            NotificationBlock(text = "Robin", kind = NotificationBlockKind.User),
            NotificationBlock(
                text = "and 10 others liked your post",
                ranges = listOf(
                    NotificationRange(
                        kind = NotificationRangeKind.Link,
                        indices = NotificationIndices(4u, 13u),
                        url = "https://garden.example.com/likes"
                    )
                )
            ),
        ),
        meta = NotificationMeta(actor = NotificationActor(name = "Sam"), actorCount = 12uL)
    )

    @Test
    fun titlesLikesWithTheirCount() {
        val layout = notificationLayout(note)
        assertNull(layout.fallback)

        val title = assertIs<NotificationLayoutText.Label>(layout.title).label
        assertEquals(NotificationLabel.LikesOnYourPost(count = 12uL), title)
        assertEquals("⁨" + "12⁩ likes on your post", localizeNotificationLabel(title, "en-US"))
        assertEquals("⁨" + "12⁩ likes on your post", title.localizedDescription(listOf(Locale.US)))
    }

    @Test
    fun listsThePeopleAndCountsTheRest() {
        val reactions = assertIs<NotificationLayoutContent.Reactions>(notificationLayout(note).content).layout
        assertTrue(reactions.isPeopleList)
        assertEquals(listOf(NotificationBlockRole.USER, NotificationBlockRole.USER), reactions.body.map { it.role })
        assertEquals(listOf("Sam", "Robin"), reactions.body.map { block -> block.runs.joinToString("") { it.text } })

        val others = assertNotNull(reactions.others)
        assertEquals(10uL, others.count)
        assertEquals("https://garden.example.com/likes", others.url)
        assertEquals("And ⁨" + "10⁩ others liked your post.", localizeNotificationLabel(others.label, "en-US"))
    }
}
