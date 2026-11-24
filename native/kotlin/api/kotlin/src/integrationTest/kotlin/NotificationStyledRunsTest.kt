package rs.wordpress.api.kotlin

import org.junit.jupiter.api.Test
import uniffi.wp_api.NotificationBlock
import uniffi.wp_api.NotificationIndices
import uniffi.wp_api.NotificationRange
import uniffi.wp_api.NotificationRangeKind
import uniffi.wp_api.TextRunLink
import uniffi.wp_api.TextRunStyle
import uniffi.wp_api.notificationBlockStyledRuns
import java.awt.font.TextAttribute
import java.text.AttributedString
import kotlin.test.assertEquals

class NotificationStyledRunsTest {
    // "Sam 😀 liked your post": a person, then a post, with bold inside the post's link.
    private val block = NotificationBlock(
        text = "Sam 😀 liked your post",
        ranges = listOf(
            NotificationRange(
                kind = NotificationRangeKind.User,
                indices = NotificationIndices(0u, 3u),
                url = "https://sam.example.org",
                id = "701"
            ),
            NotificationRange(
                kind = NotificationRangeKind.Post,
                indices = NotificationIndices(18u, 22u),
                id = "601",
                siteId = 801uL
            ),
            NotificationRange(kind = NotificationRangeKind.Bold, indices = NotificationIndices(18u, 22u)),
        )
    )

    @Test
    fun buildsAttributedText() {
        // Offsets count UTF-16 units, as a Kotlin `String` does, so they index the block's text.
        val text = AttributedString(block.text)
        for (run in notificationBlockStyledRuns(block)) {
            if (TextRunStyle.Bold in run.styles) {
                val start = run.utf16Offset.toInt()
                val end = start + run.utf16Length.toInt()
                text.addAttribute(TextAttribute.WEIGHT, TextAttribute.WEIGHT_BOLD, start, end)
            }
        }

        val iterator = text.iterator
        iterator.index = block.text.indexOf("post")
        assertEquals(TextAttribute.WEIGHT_BOLD, iterator.getAttribute(TextAttribute.WEIGHT))
        assertEquals("post".length, iterator.runLimit - iterator.index)
    }

    @Test
    fun readsRuns() {
        val runs = notificationBlockStyledRuns(block)

        assertEquals(listOf("Sam", " 😀 liked your ", "post"), runs.map { it.text })
        assertEquals(block.text, runs.joinToString("") { it.text })
        assertEquals(listOf<TextRunStyle>(TextRunStyle.Bold), runs.last().styles)
        assertEquals(TextRunLink.Post(siteId = 801uL, postId = 601L, url = null), runs.last().link)
        assertEquals(
            TextRunLink.User(userId = 701uL, siteId = null, url = "https://sam.example.org"),
            runs.first().link
        )
    }
}
