import Foundation
import Testing
import WordPressAPI
import WordPressAPIInternal

class LocalizationTests {
    @Test
    func testParsingError() {
        do {
            _ = try ParsedUrl.parse(input: "not-url")
            Issue.record("Got an unexpected successful result")
        } catch {
            #expect(error.localizedDescription == "URL is invalid.")
        }
    }

    @Test
    func testAuthorizationHeaderBlocked() throws {
        let error = VerifyIssuedApplicationPasswordError.AuthorizationHeaderBlocked(
            hostname: "example.com",
            error: .UnknownError(statusCode: 401, response: "", requestUrl: "https://example.com", requestMethod: .get)
        )
        #expect(error.localizedDescription == """
            Sign-in could not complete because your site's server is not passing login credentials to WordPress. \
            Contact your hosting provider about this error. \
            Your site's Site Health page should list it as “The authorization header is missing”.
            """)
    }
}
