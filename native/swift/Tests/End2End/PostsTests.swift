#if os(macOS) || os(Linux)

import XCTest
import wordpress_api

class PostsTests: XCTestCase {
    func testGetPost() async throws {
        let view = try await site.api.posts.forViewing.get(id: 1)
        XCTAssertEqual(view.title.rendered, "Hello world!")

        let edit = try await site.api.posts.forEditing.get(id: 1)
        XCTAssertTrue(edit.content.raw.contains("<!-- wp:paragraph -->"))

        let embed = try await site.api.posts.forEmbedding.get(id: 1)
        XCTAssertTrue(embed.excerpt.rendered.contains("<p>"))
    }
}

#endif
