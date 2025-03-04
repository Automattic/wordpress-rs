import Foundation
import Testing
import WordPressAPI
import WordPressAPIInternal

class LocalizationTests {
    @Test
    func testParsingError() {
        do {
            let _ = try ParsedUrl.parse(input: "not-url")
            Issue.record("Got an unexpected successful result")
        } catch {
            #expect(error.localizedDescription == "URL is invalid")
        }
    }
}
