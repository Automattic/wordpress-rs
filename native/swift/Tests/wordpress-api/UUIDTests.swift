import Foundation
import XCTest
import WordPressAPI

class WPUUIDTests: XCTestCase {

    func testConvertToUUID() {
        let uuid = WpUuid().uuidString()
        XCTAssertNotNil(UUID(uuidString: uuid), "WpUuid \(uuid) is not a Foundation.UUID")
    }

}
