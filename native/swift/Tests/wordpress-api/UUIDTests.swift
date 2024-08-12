import Foundation
import XCTest
import WordPressAPI

class UUIDTests: XCTestCase {

    func testConvertToUUID() {
        let uuid = WpUuid().uuidString()
        XCTAssertNotNil(UUID(uuidString: uuid), "WpUuid \(uuid) is not a Foundation.UUID")
    }

}
