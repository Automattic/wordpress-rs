import Foundation
import WordPressAPI
import Testing

@Suite
struct BlockSettingsTests {
    let api = WordPressAPI.admin()

    @Test
    func fetchRawBlockSettings() async throws {
        let response = try await api.blockEditor.getRawSettings(params: WpBlockEditorSettingsParams())
        #expect(!response.data.payload.isEmpty)
    }
}
