import Foundation
@testable import WordPressAPI
import Testing

@Suite
struct MediaTests {
    let api = WordPressAPI.admin()

    @Test
    func uploadImage() async throws {
        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        let response = try await api.media.create(
            params: .init(title: "Image", altText: "This is a test image"),
            filePath: file.path,
            fileContentType: "image/jpeg",
            requestId: nil,
            cancellationToken: nil
        )
        #expect(response.data.mimeType == "image/jpeg")
        #expect(response.data.title.raw == "Image")
        #expect(response.data.altText == "This is a test image")

        try await restoreTestServer()
    }

#if os(macOS)
    @Test
    func uploadProgress() async throws {
        let progress = Progress.discreteProgress(totalUnitCount: 100)
        #expect(progress.fractionCompleted == 0)

        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        let response = try await api.uploadMedia(
            params: .init(),
            fromLocalFileURL: file,
            fulfilling: progress
        )
        #expect(response.data.mimeType == "image/jpeg")
        #expect(progress.fractionCompleted == 1)

        try await restoreTestServer()
    }

    @Test
    func cancelProgress() async throws {
        let progress = Progress.discreteProgress(totalUnitCount: 100)
        #expect(progress.fractionCompleted == 0)

        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        await #expect(
            throws: WpApiError.RequestExecutionFailed(statusCode: nil, redirects: nil, reason: .cancellationError),
            performing: {
                let task = Task {
                    _ = try await api.uploadMedia(
                        params: .init(),
                        fromLocalFileURL: file,
                        fulfilling: progress
                    )
                }

                let cancellable = progress.publisher(for: \.fractionCompleted).first { $0 > 0 }.sink { _ in
                    progress.cancel()
                }
                defer { cancellable.cancel() }

                try await task.value
            }
        )

        try await restoreTestServer()
    }

    @Test
    func cancelTask() async throws {
        let progress = Progress.discreteProgress(totalUnitCount: 100)
        #expect(progress.fractionCompleted == 0)
        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        await #expect(
            throws: WpApiError.RequestExecutionFailed(statusCode: nil, redirects: nil, reason: .cancellationError),
            performing: {
                let task = Task {
                    _ = try await api.uploadMedia(
                        params: .init(),
                        fromLocalFileURL: file,
                        fulfilling: progress
                    )
                }

                let cancellable = progress.publisher(for: \.fractionCompleted).first { $0 > 0 }.sink { _ in
                    task.cancel()
                }
                defer { cancellable.cancel() }

                try await task.value
            }
        )

        try await restoreTestServer()
    }

    @Test
    func cancelCreateMediaTask() async throws {
        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        await #expect(
            throws: WpApiError.RequestExecutionFailed(statusCode: nil, redirects: nil, reason: .cancellationError),
            performing: {
                let requestId = WpUuid()
                let task = Task {
                    _ = try await api.media.create(
                        params: .init(),
                        filePath: file.path(),
                        fileContentType: "image/jpg",
                        requestId: requestId,
                        cancellationToken: nil
                    )
                }

                let progress = try await api.requestExecutor
                    .progress(forRequestWithId: requestId.uuidString())
                    .values
                    .first { _ in true }
                let cancellable = progress!
                    .publisher(for: \.fractionCompleted)
                    .first { $0 > 0 }
                    .sink { _ in task.cancel() }
                defer { cancellable.cancel() }

                try await task.value
            }
        )

        try await restoreTestServer()
    }

    @Test
    func cancelNewCreateMediaTask() async throws {
        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        await #expect(
            throws: WpApiError.RequestExecutionFailed(statusCode: nil, redirects: nil, reason: .cancellationError),
            performing: {
                let requestId = WpUuid()
                let task = Task {
                    _ = try await api.createMedia(
                        params: .init(),
                        filePath: file.path(),
                        fileContentType: "image/jpg",
                        requestId: requestId
                    )
                }

                let progress = try await api.requestExecutor
                    .progress(forRequestWithId: requestId.uuidString())
                    .values
                    .first { _ in true }
                let cancellable = progress!.publisher(for: \.fractionCompleted).first { $0 > 0 }.sink { _ in
                    task.cancel()
                }
                defer { cancellable.cancel() }

                try await task.value
            }
        )

        try await restoreTestServer()
    }
#endif
}
