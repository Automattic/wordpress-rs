import Foundation
import Testing

@testable import WordPressAPI
@testable import WordPressAPIInternal

struct CancellationTests {
    let api = WordPressAPI.admin()

    @Test
    func cancelUploadingLongPost() async throws {
        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        let content = try String(data: Data(contentsOf: file).base64EncodedData(), encoding: .utf8)!

        let title = UUID().uuidString
        await #expect(
            throws: WpApiError.RequestExecutionFailed(statusCode: nil, redirects: nil, reason: .cancellationError),
            performing: {
                let task = Task {
                    _ = try await api.posts.create(params: .init(title: title, content: content, meta: nil))
                    Issue.record("The creating post function should throw")
                }

                try await Task.sleep(for: .milliseconds(10))
                task.cancel()

                try await task.value
            }
        )

        try await restoreTestServer()
    }
}
