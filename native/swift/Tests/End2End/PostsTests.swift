#if os(macOS) || os(Linux)

import XCTest
import wordpress_api

class PostsTests: XCTestCase {
    func testGetPost() async throws {
        let view = try await site.api.posts.forViewing.get(id: 1)
        XCTAssertNil(view.content.raw)

        let edit = try await site.api.posts.forEditing.get(id: 1)
        XCTAssertEqual(edit.content.raw?.contains("<!-- wp:paragraph -->"), true)

        let embed = try await site.api.posts.forEmbedding.get(id: 1)
        XCTAssertNil(view.content.raw)
        XCTAssertEqual(embed.excerpt.rendered?.contains("<p>"), true)
    }
}

#endif
