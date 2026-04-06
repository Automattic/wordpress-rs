import Foundation
import Testing
import WordPressAPI

// This test ensures that existing code compiles – it's not meant to run anything
struct PostsCompatTests {

    @Test func `test post update params`() async throws {
        _ = PostUpdateParams(
            title: "Hello World",
            content: "Updated content",
            meta: nil
        )
    }
}
