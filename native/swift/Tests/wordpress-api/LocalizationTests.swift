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

    // `scripts/swift-bindings.sh` adds the `LocalizedError` conformance by grepping for a literal
    // `impl WpSupportsLocalization for AutoDiscoveryAttemptFailure`. If that impl stops being
    // hand-written, the conformance silently disappears and `localizedDescription` falls back to
    // Foundation's "The operation couldn't be completed" text. The tests below guard against that.

    @Test
    func testDiscoveryErrorUsesRustMessage() throws {
        let failure = AutoDiscoveryAttemptFailure.FindApiRoot(
            parsedSiteUrl: try ParsedUrl.parse(input: "https://example.com"),
            findApiRootFailure: .restApiDisabled
        )

        #expect(failure.localizedDescription == localizedForPreferredLanguages(failure))
    }

    @Test
    func testDiscoveryRestApiDisabledMessage() throws {
        let failure = AutoDiscoveryAttemptFailure.FindApiRoot(
            parsedSiteUrl: try ParsedUrl.parse(input: "https://example.com"),
            findApiRootFailure: .restApiDisabled
        )

        #expect(
            englishMessage(failure)
                == "The site's REST API is disabled. Please update your site settings to enable REST API."
        )
    }

    @Test
    func testDiscoveryProbablyNotWordPressMessage() throws {
        let failure = AutoDiscoveryAttemptFailure.FindApiRoot(
            parsedSiteUrl: try ParsedUrl.parse(input: "https://example.com"),
            findApiRootFailure: .probablyNotAWordPressSite
        )

        #expect(englishMessage(failure) == "The site does not appear to be a WordPress site.")
    }

    @Test
    func testDiscoveryInvalidSslMessage() throws {
        let failure = AutoDiscoveryAttemptFailure.FindApiRoot(
            parsedSiteUrl: try ParsedUrl.parse(input: "https://example.com"),
            findApiRootFailure: .fetchHomepage(
                error: .RequestExecutionFailed(
                    statusCode: nil,
                    redirects: nil,
                    reason: .invalidSslError(reason: .genericSslError),
                    requestUrl: "https://example.com",
                    requestMethod: .get
                )
            )
        )

        #expect(
            englishMessage(failure)
                == "Unable to establish a secure connection to the site. Its SSL certificate may be invalid or expired."
        )
    }

    @Test
    func testDiscoveryUnreadableApiRootMessage() throws {
        let failure = AutoDiscoveryAttemptFailure.FetchAndParseApiRoot(
            parsedSiteUrl: try ParsedUrl.parse(input: "https://example.com"),
            apiRootUrl: try ParsedUrl.parse(input: "https://example.com/wp-json"),
            fetchAndParseApiRootFailure: .parseApiRoot(
                parsingErrorMessage: "expected value at line 1 column 1",
                responseBody: "<html></html>",
                responseBodyType: .maybeHtml,
                reason: nil
            )
        )

        #expect(
            englishMessage(failure)
                == "Found the site, but couldn't read its WordPress REST API response. "
                + "A plugin or server configuration may be interfering with the REST API."
        )
    }

    @Test
    func testDiscoveryPrivateSiteRendersServerMessage() throws {
        let failure = AutoDiscoveryAttemptFailure.FetchAndParseApiRoot(
            parsedSiteUrl: try ParsedUrl.parse(input: "https://private.example.com"),
            apiRootUrl: try ParsedUrl.parse(input: "https://private.example.com/wp-json"),
            fetchAndParseApiRootFailure: .wpError(
                errorCode: .Forbidden,
                errorMessage: "This site is private.",
                statusCode: 401
            )
        )

        #expect(englishMessage(failure) == "This site is private.")
    }

    private func englishMessage(_ failure: AutoDiscoveryAttemptFailure) -> String {
        localizeAutoDiscoveryAttemptFailure(value: failure, locale: wpLocaleResolve(langIds: ["en-US"]))
    }

    private func localizedForPreferredLanguages(_ failure: AutoDiscoveryAttemptFailure) -> String {
        localizeAutoDiscoveryAttemptFailure(
            value: failure,
            locale: wpLocaleResolve(langIds: Locale.preferredLanguages)
        )
    }
}
