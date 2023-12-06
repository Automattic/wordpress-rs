import XCTest

import wordpress_api

final class WordPressAPITests: XCTestCase {

    func testThatCombiningStringsWorks() throws {
        XCTAssertEqual(WordPressAPI().combineStrings("Hello", "World"), "Hello-World")
    }

//  Future Test:
//    func testThatRequestSkeletonWorks() async throws {
//        let response = try await WordPressAPI().listPosts(params: .init(page: 1, perPage: 99))
//        XCTAssertTrue(try XCTUnwrap(response.postList).isEmpty)
//    }
}
