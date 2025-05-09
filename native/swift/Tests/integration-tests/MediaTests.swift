import Foundation
import WordPressAPI
import Testing
import AppKit

@Suite
struct MediaTests {
    let api = WordPressAPI.admin()

    @Test
    func uploadImage() async throws {
        let file = try #require(Bundle.module.url(forResource: "test-data/test_media.jpg", withExtension: nil))
        let response = try await api.uploadMedia(params: .init(), fromLocalFileURL: file, fulfilling: Progress.discreteProgress(totalUnitCount: 100))
        #expect(response.data.mimeType == "image/jpeg")

        try await restoreTestServer()
    }
}
