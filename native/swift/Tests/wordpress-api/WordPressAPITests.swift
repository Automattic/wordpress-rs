import XCTest
import Foundation

import wordpress_api

final class WordPressAPITests: XCTestCase {

    let api = WordPressAPI(
        urlSession: .shared,
        authenticationStategy: .httpBasic(username: "user", password: "password")
    )

    func testThatCombiningStringsWorks() throws {
        XCTAssertEqual(api.combineStrings("Hello", "World"), "Hello-World")
    }

    func testThatListRequestSkeletonWorks() async throws {
        let response = try await api.listPosts(params: .init(page: 1, perPage: 99))
        XCTAssertTrue(response.isEmpty)
    }

//    Current Fails with "Not Implemented"
//    func testThatRetrieveRequestSkeletonWorks() async throws {
//        let response = try await api.retrievePost(id: 42)
//        XCTAssertNil(response)
//    }

    func testThatCreateRequestSkeletonWorks() async throws {
        let response = try await api.createPost(params: .init(title: "My Post", content: "With Content"))
        XCTAssertEqual(response.id, 1)
        XCTAssertEqual(response.title, "foo")
        XCTAssertEqual(response.content, "bar")
    }
}
