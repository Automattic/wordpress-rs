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

    func testDelete() async throws {
        let newPost = try await site.api.posts.create(using: .init(title: "Test post", content: "This is a test post"))
        try await site.api.posts.delete(id: newPost.id, params: .init(force: true))
        do {
            _ = try await site.api.posts.forViewing.get(id: newPost.id)
            XCTFail("The post should have been deleted")
        } catch {
            // Do nothing
        }
    }

    func testUpdate() async throws {
        let newPost = try await site.api.posts.create(using: .init(title: "Test post", content: "This is a test post"))
        _ = try await site.api.posts.update(id: newPost.id, with: .init(title: "Updated", content: nil))

        let updated = try await site.api.posts.forViewing.get(id: newPost.id)
        XCTAssertEqual(updated.title.rendered, "Updated")
    }
}

#endif
