import XCTest
import Foundation
import WordPressAPI

final class WordPressAPITests: XCTestCase {

    func testExample() async throws {
        // swiftlint:disable todo
        // TODO: Implement a `RequestExecutor` to stub HTTP requests in unit tests.
        // swiftlint:enable todo
        let api = try WordPressAPI(
            urlSession: .shared,
            baseUrl: URL(string: "https://wordpress.org")!,
            authenticationStategy: .none
        )
        let users = try await api.users.listWithViewContext(params: nil)
        XCTAssertFalse(users.isEmpty)
    }
}
