import Foundation
import WordPressAPI
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
            requestId: nil
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
    #endif
}
