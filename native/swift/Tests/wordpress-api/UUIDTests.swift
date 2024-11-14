import Foundation
import Testing
import WordPressAPI

struct WPUUIDTests {

    @Test
    func testConvertToUUID() {
        let uuid = WpUuid().uuidString()
        #expect(UUID(uuidString: uuid) != nil, "WpUuid \(uuid) is not a Foundation.UUID")
    }
}
