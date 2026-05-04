import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

class HTTPErrorTests {

    @Test
    func testTimeout() async throws {
        let stubs = HTTPStubs(stubs: [], missingStub: .failure(URLError(.timedOut)))

        let api = try WordPressAPI(
            siteInfo: .selfHosted(
                siteUrl: ParsedUrl.parse(input: "https://wordpress.org"),
                apiRoot: ParsedUrl.parse(input: "https://wordpress.org/wp-json")
            ),
            authenticationProvider: .none(),
            executor: stubs,
            middlewarePipeline: .default,
            appNotifier: nil
        )

        await #expect(
            throws: WpApiError.self,
            performing: {
                _ = try await api.users.retrieveWithViewContext(userId: 1)
            }
        )
    }
}
