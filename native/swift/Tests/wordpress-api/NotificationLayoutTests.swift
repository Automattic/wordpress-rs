import Foundation
import Testing
import WordPressAPI

struct NotificationLayoutTests {

    // Twelve people liked a post: the note lists two of them, then a line about the rest.
    let note = WpComNotification(
        id: 10_000_000_002,
        noteType: .like,
        title: "Likes",
        url: "https://garden.example.com/winter",
        body: [
            NotificationBlock(text: "Sam", kind: .user),
            NotificationBlock(text: "Robin", kind: .user),
            NotificationBlock(
                text: "and 10 others liked your post",
                ranges: [
                    NotificationRange(
                        kind: .link,
                        indices: NotificationIndices(start: 4, end: 13),
                        url: "https://garden.example.com/likes"
                    )
                ]
            )
        ],
        meta: NotificationMeta(actor: NotificationActor(name: "Sam"), actorCount: 12)
    )

    @Test
    func titlesLikesWithTheirCount() {
        let layout = note.layout
        #expect(layout.fallback == nil)

        guard case .label(let title)? = layout.title else {
            Issue.record("Expected a label for the title")
            return
        }
        #expect(title == .likesOnYourPost(count: 12))
        #expect(title.localizedText(languages: ["en-US"]) == "\u{2068}12\u{2069} likes on your post")
    }

    @Test
    func listsThePeopleAndCountsTheRest() throws {
        guard case .reactions(let reactions) = note.layout.content else {
            Issue.record("Expected the reactions layout")
            return
        }
        #expect(reactions.isPeopleList)
        #expect(reactions.body.map(\.role) == [.user, .user])
        #expect(reactions.body.map { $0.runs.map(\.text).joined() } == ["Sam", "Robin"])

        let others = try #require(reactions.others)
        #expect(others.count == 10)
        #expect(others.url == "https://garden.example.com/likes")
        #expect(others.label.localizedText(languages: ["en-US"]) == "And \u{2068}10\u{2069} others liked your post.")
    }

    @Test
    func fallsBackToANoticeWithoutContext() {
        let note = WpComNotification(id: 1, noteType: .like, title: "Likes", body: [NotificationBlock(text: "Sam")])
        let layout = note.layout
        #expect(layout.fallback == .noContext)
        #expect(layout.title == .text(text: "Likes"))
        guard case .notice(let notice) = layout.content else {
            Issue.record("Expected the notice layout")
            return
        }
        #expect(notice.body.map(\.role) == [.text])
    }
}
