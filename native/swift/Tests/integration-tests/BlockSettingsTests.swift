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

        let json = try response.data.asJson()
        if let dict = json as? [String: Any] {
            #expect(dict["alignWide"] as? Bool == false, "alignWide should be false in test environment")
        } else {
            Issue.record("Expected JSON object but got \(type(of: json))")
        }
    }
}
