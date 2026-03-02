import Foundation
import Testing

@testable import WordPressAPI
@testable import WordPressAPIInternal

#if os(macOS)

@Suite(.serialized)
struct CancellationTests {
    let api = WordPressAPI.admin()

    @Test
    func cancelUploadingLongPost() async throws {
        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        let content = try String(data: Data(contentsOf: file).base64EncodedData(), encoding: .utf8)!

        let title = UUID().uuidString
        let error = await #expect(throws: WpApiError.self, performing: {
                let task = Task {
                    let params = PostCreateParams(
                        title: title,
                        content: content,
                        meta: nil
                    )
                    _ = try await api.posts.create(postEndpointType: .posts, params: params)
                    Issue.record("The creating post function should throw")
                }

                try await Task.sleep(for: .milliseconds(10))
                task.cancel()

                try await task.value
            }
        )
        #expect(error?.isCancellationError == true)

        try await restoreTestServer()
    }
}

#endif
