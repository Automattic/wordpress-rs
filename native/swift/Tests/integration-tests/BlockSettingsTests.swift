import Foundation
import WordPressAPI
import Testing

@Suite(.serialized)
struct BlockSettingsTests {
    let api = WordPressAPI.admin()

    @Test
    func fetchRawBlockSettings() async throws {
        let response = try await api.blockEditor.retrieveSettings(params: WpBlockEditorSettingsParams())
        #expect(response.data.payload.asBytes().count > 0)

        let jsonRoot = response.data.payload.asJson()

        guard case .object(let dictionary) = jsonRoot, case .bool(let value) = dictionary["alignWide"] else {
            Issue.record("Invalid JSON")
            return
        }

        #expect(value == false, "alignWide should be false in test environment")
    }
}
