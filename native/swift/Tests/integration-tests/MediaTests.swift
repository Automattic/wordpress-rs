import Foundation
import Testing
import WordPressApiCache
@testable import WordPressAPI

@Suite(.serialized)
struct MediaTests {
    let api = WordPressAPI.admin()

    @Test
    func uploadImage() async throws {
        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        let response = try await api.media.create(
            params: .init(title: "Image", altText: "This is a test image", filePath: file.path)
        )
        #expect(response.data.mimeType == "image/jpeg")
        #expect(response.data.title.raw == "Image")
        // It appears this particular assertion is flaky on CI, and only on Linux containers.
        // Here is an example: https://buildkite.com/automattic/wordpress-rs/builds/4003
        // #expect(response.data.altText == "This is a test image")

        try await restoreTestServer()
    }

    @Test
    func fileNotFoundError() async throws {
        let file = "/path/to/a/non-existent-file.jpg"
        await #expect(
            throws: WpApiError.MediaFileNotFound(filePath: file),
            performing: {
                _ = try await api.media.create(params: .init(filePath: file))
            }
        )

        try await restoreTestServer()
    }

    #if os(macOS)
    @Test
    func uploadProgress() async throws {
        let progress = Progress.discreteProgress(totalUnitCount: 100)
        #expect(progress.fractionCompleted == 0)

        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        let response = try await api.uploadMedia(
            params: .init(filePath: file.path),
            fulfilling: progress
        )
        #expect(response.data.mimeType == "image/jpeg")
        #expect(progress.fractionCompleted == 1)

        try await restoreTestServer()
    }

    @Test
    func uploadProgressWithService() async throws {
        let cache = try WordPressApiCache()
        _ = try cache.performMigrations()
        let service = try api.createService(cache: cache)

        let progress = Progress.discreteProgress(totalUnitCount: 100)
        #expect(progress.fractionCompleted == 0)

        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        let response = try await service.uploadMedia(
            params: .init(filePath: file.path),
            fulfilling: progress
        )
        #expect(response.mimeType == "image/jpeg")
        #expect(progress.fractionCompleted == 1)

        try await restoreTestServer()
    }

    @Test
    func cancelProgress() async throws {
        let progress = Progress.discreteProgress(totalUnitCount: 100)
        #expect(progress.fractionCompleted == 0)

        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        let error = await #expect(
            throws: WpApiError.self,
            performing: {
                let task = Task {
                    _ = try await api.uploadMedia(
                        params: .init(filePath: file.path),
                        fulfilling: progress
                    )
                    Issue.record("The creating post function should throw")
                }

                let cancellable = progress.publisher(for: \.fractionCompleted).first { $0 > 0 }
                    .sink { _ in
                        progress.cancel()
                    }
                defer { cancellable.cancel() }

                try await task.value
            }
        )
        #expect(error?.isCancellationError == true)

        try await restoreTestServer()
    }

    @Test
    func cancelTask() async throws {
        let progress = Progress.discreteProgress(totalUnitCount: 100)
        #expect(progress.fractionCompleted == 0)
        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        let error = await #expect(
            throws: WpApiError.self,
            performing: {
                let task = Task {
                    _ = try await api.uploadMedia(
                        params: .init(filePath: file.path),
                        fulfilling: progress
                    )
                    Issue.record("The creating post function should throw")
                }

                let cancellable = progress.publisher(for: \.fractionCompleted).first { $0 > 0 }
                    .sink { _ in
                        task.cancel()
                    }
                defer { cancellable.cancel() }

                try await task.value
            }
        )
        #expect(error?.isCancellationError == true)

        try await restoreTestServer()
    }
    #endif
}
