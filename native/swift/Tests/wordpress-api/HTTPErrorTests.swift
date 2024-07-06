import Foundation
import XCTest

@testable import WordPressAPI

#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

class HTTPErrorTests: XCTestCase {

#if !os(Linux)
    // Skip on Linux, because `XCTExpectFailure` is unavailable on Linux
    func testTimeout() async throws {
        let stubs = HTTPStubs()
        stubs.missingStub = .failure(URLError(.timedOut))

        let api = try WordPressAPI(
            urlSession: .shared,
            baseUrl: ParsedUrl.parse(input: "https://wordpress.org"),
            authenticationStategy: .none,
            executor: stubs
        )

        do {
            _ = try await api.users.retrieveWithViewContext(userId: 1)
            XCTFail("Unexpected response")
        } catch let error as URLError {
            XCTAssertEqual(error.code, .timedOut)
        } catch {
            #if canImport(WordPressAPIInternal)
            XCTAssertTrue(error is WordPressAPIInternal.WpApiError)
            #else
            XCTAssertTrue(error is WpApiError)
            #endif
        }
    }
#endif

}
