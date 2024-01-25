import XCTest
import Foundation

import wordpress_api

final class WordPressAPITests: XCTestCase {

    let api = WordPressAPI(
        urlSession: .shared,
        baseUrl: URL(string: "https://sweetly-unadulterated.jurassic.ninja")!,
        authenticationStategy: .init(username: "demo", password: "qD6z ty5l oLnL gXVe 0UED qBUB")
    )

    func testThatListRequestReturnsPosts() async throws {
        let response = try await api.listPosts(params: .init(page: 1, perPage: 99))
        XCTAssertFalse(try XCTUnwrap(response.postList?.isEmpty))
    }

    func testThatListRequestReturnsCorrectNumberOfPostsByDefault() async throws {
        let response = try await api.listPosts()
        XCTAssertEqual(response.postList?.count, 10)
    }

    func testThatListRequestReturnsCorrectNumberOfPostsWhenSpecified() async throws {
        let response = try await api.listPosts(params: .init(page: 1, perPage: 25))
        XCTAssertEqual(response.postList?.count, 25)
    }
}
