import Foundation
import Testing
import WordPressAPI

struct NotificationStyledRunsTests {

    // "Sam 😀 liked your post": a person, then a post, with bold inside the post's link.
    let block = NotificationBlock(
        text: "Sam 😀 liked your post",
        ranges: [
            NotificationRange(
                kind: .user,
                indices: NotificationIndices(start: 0, end: 3),
                url: "https://sam.example.org",
                id: "701"
            ),
            NotificationRange(
                kind: .post,
                indices: NotificationIndices(start: 18, end: 22),
                id: "601",
                siteId: 801
            ),
            NotificationRange(kind: .bold, indices: NotificationIndices(start: 18, end: 22))
        ]
    )

    // `AttributedString` has no `inlinePresentationIntent` in Linux's Foundation.
    #if !os(Linux)
    @Test
    func buildsAttributedText() throws {
        var text = AttributedString()
        for run in block.styledRuns {
            var part = AttributedString(run.text)
            if run.styles.contains(.bold) {
                part.inlinePresentationIntent = .stronglyEmphasized
            }
            if case .web(let url)? = run.link {
                part.link = URL(string: url)
            }
            text.append(part)
        }

        #expect(String(text.characters) == block.text)
        let bold = try #require(text.range(of: "post"))
        #expect(text[bold].inlinePresentationIntent == .stronglyEmphasized)
    }
    #endif

    @Test
    func readsRuns() throws {
        let runs = block.styledRuns
        #expect(runs.map(\.text) == ["Sam", " 😀 liked your ", "post"])

        // Offsets count UTF-16 units, as `NSString` does: the emoji takes two.
        let post = try #require(runs.last)
        #expect(post.utf16Offset == 18)
        #expect((block.text as NSString).substring(with: NSRange(location: 18, length: 4)) == "post")
        #expect(post.styles == [.bold])
        #expect(post.link == .post(siteId: 801, postId: 601, url: nil))
        #expect(runs.first?.link == .user(userId: 701, siteId: nil, url: "https://sam.example.org"))
    }
}
