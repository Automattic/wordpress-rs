import Foundation
import Testing

@testable import WordPressAPI
@testable import WordPressAPIInternal

@Suite(.disabled("The tests fails due to issues in uniffi-rs"))
struct KnownIssues {
    let api = WordPressAPI.admin()

    @Test
    func uniffiAsyncFunctionsAreNotCancellable() async throws {
        // See https://mozilla.github.io/uniffi-rs/0.29/futures.html#cancelling-async-code

        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        let task = Task {
            _ = try await api.media.create(
                params: .init(title: "Image", altText: "This is a test image"),
                filePath: file.path,
                fileContentType: "image/jpeg",
                requestId: nil,
                cancellationToken: nil
            )
        }

        try await Task.sleep(nanoseconds: 50_000_000)
        task.cancel()

        await #expect(
            throws: WpApiError.RequestExecutionFailed(statusCode: nil, redirects: nil, reason: .cancellationError),
            performing: {
                try await task.value
            }
        )

        try await restoreTestServer()
    }
}
