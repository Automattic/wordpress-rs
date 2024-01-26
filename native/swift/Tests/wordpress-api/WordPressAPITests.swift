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

    func testThatListRequestFetchesMaxCount() async throws {
        let response = try await api.listPosts(params: .init(page: 1, perPage: 100))
        XCTAssertEqual(response.postList?.count, 36)
    }

    func testThatNextLinkIsNotNilWhenFetchingLessThanAllPosts() async throws {
        let response = try await api.listPosts()
        XCTAssertNotNil(response.nextPage)
    }

    func testThatFetchingAllPagesWorks() async throws {
        let response = try await api.listPosts()
        let nextPage = try XCTUnwrap(response.nextPage)
        XCTAssertEqual(response.postList?.count, 10)

        let response2 = try await api.listPosts(url: nextPage)
        let nextPage2 = try XCTUnwrap(response2.nextPage)
        XCTAssertEqual(response2.postList?.count, 10)

        let response3 = try await api.listPosts(url: nextPage2)
        let nextPage3 = try XCTUnwrap(response3.nextPage)
        XCTAssertEqual(response3.postList?.count, 10)

        let response4 = try await api.listPosts(url: nextPage3)
        XCTAssertNil(response4.nextPage)
        XCTAssertEqual(response4.postList?.count, 6)
    }

    func testThatFetchingAllPagesWithAsyncIteratorWorks() async throws {
        var posts = PostCollection()

        for try await post in api.listPosts() {
            posts.append(post)
        }

        XCTAssertEqual(posts.count, 36)
    }
}
