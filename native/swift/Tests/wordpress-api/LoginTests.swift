import Foundation
import XCTest

@testable import WordPressAPI

#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

class LoginTests: XCTestCase {

    var stubs: HTTPStubs!

    override func setUp() {
        super.setUp()
        stubs = HTTPStubs()
        stubs.missingStub = .failure(URLError(.timedOut))
    }

    func testInvalidUrl() async {
        let client = WordPressLoginClient(requestExecutor: stubs)
        do {
            let result = await client.login(
                site: "invalid url",
                appName: "foo",
                appId: "bar",
                authenticator: Authenticator()
            )
            let success = try result.get()
            XCTFail("Unexpected successful result: \(success)")
        } catch WordPressLoginClient.Error.invalidSiteAddress {
            // Do nothing
        } catch {
            XCTFail("Unexpected error: \(error)")
        }
    }

    func testNotWordPressSite() async throws {
        try stubs.stub(
            host: "example.com",
            with: WpNetworkResponse(body: Data(), statusCode: 200, headerMap: .fromMap(hashMap: [:]))
        )
        let client = WordPressLoginClient(requestExecutor: stubs)
        do {
            let result = await client.login(
                site: "https://example.com/blog",
                appName: "foo",
                appId: "bar",
                authenticator: Authenticator()
            )
            let success = try result.get()
            XCTFail("Unexpected successful result: \(success)")
        } catch let WordPressLoginClient.Error.invalidSiteAddress(error) {
            switch error {
            case let .UrlDiscoveryFailed(attempts: attempts):
                let notWordPressSiteError = attempts.values.contains {
                    if case .failure(.fetchApiRootUrlFailed) = $0 {
                        return true
                    }
                    return false
                }

                XCTAssertTrue(notWordPressSiteError, "Error is not 'fetchApiRootUrlFailed': \(error)")
            }
        } catch {
            XCTFail("Unexpected error: \(error)")
        }
    }

    func testWpJsonError() async throws {
        try stubs.stub(
            url: "https://example.com/",
            with: WpNetworkResponse(
                body: Data(),
                statusCode: 200,
                headerMap: .fromMap(hashMap: ["Link": #"<https://example.com/wp-json/>; rel="https://api.w.org/""#])
            )
        )
        try stubs.stub(
            url: "https://example.com/wp-json/",
            with: WpNetworkResponse(
                body: "not a json".data(using: .utf8)!,
                statusCode: 200,
                headerMap: .fromMap(hashMap: ["Link": #"<https://example.com/wp-json/>; rel="https://api.w.org/""#])
            )
        )

        let client = WordPressLoginClient(requestExecutor: stubs)
        do {
            let result = await client.login(
                site: "https://example.com",
                appName: "foo",
                appId: "bar",
                authenticator: Authenticator()
            )
            let success = try result.get()
            XCTFail("Unexpected successful result: \(success)")
        } catch let WordPressLoginClient.Error.invalidSiteAddress(error) {
            switch error {
            case let .UrlDiscoveryFailed(attempts: attempts):
                let notWordPressSiteError = attempts.values.contains {
                    if case .failure(.fetchApiDetailsFailed) = $0 {
                        return true
                    }
                    return false
                }

                XCTAssertTrue(notWordPressSiteError, "Error is not 'fetchApiRootUrlFailed': \(error)")
            }
        } catch {
            XCTFail("Unexpected error: \(error)")
        }
    }

    func testMissingAuthenticationEndpoint() async throws {
        try stubs.stub(
            url: "https://example.com/",
            with: WpNetworkResponse(
                body: Data(),
                statusCode: 200,
                headerMap: .fromMap(hashMap: ["Link": #"<https://example.com/wp-json/>; rel="https://api.w.org/""#])
            )
        )

        let wpJsonResponse = try XCTUnwrap(
            Bundle.module.url(
                forResource: "Responses/LoginTests-wp-json-missing-authentication-endpoint",
                withExtension: "json"
            )
        )

        try stubs.stub(
            url: "https://example.com/wp-json/",
            with: WpNetworkResponse(
                body: Data(contentsOf: wpJsonResponse),
                statusCode: 200,
                headerMap: .fromMap(hashMap: ["Link": #"<https://example.com/wp-json/>; rel="https://api.w.org/""#])
            )
        )

        let client = WordPressLoginClient(requestExecutor: stubs)
        do {
            let result = await client.login(
                site: "https://example.com",
                appName: "foo",
                appId: "bar",
                authenticator: Authenticator()
            )
            let success = try result.get()
            XCTFail("Unexpected successful result: \(success)")
        } catch WordPressLoginClient.Error.missingLoginUrl {
            // Do nothing
        } catch {
            XCTFail("Unexpected error: \(error)")
        }
    }

    func testRejectedResult() async throws {
        try stubs.stub(
            url: "https://example.com/",
            with: WpNetworkResponse(
                body: Data(),
                statusCode: 200,
                headerMap: .fromMap(hashMap: ["Link": #"<https://example.com/wp-json/>; rel="https://api.w.org/""#])
            )
        )

        let wpJsonResponse = try XCTUnwrap(
            Bundle.module.url(
                forResource: "Responses/LoginTests-wp-json",
                withExtension: "json"
            )
        )

        try stubs.stub(
            url: "https://example.com/wp-json/",
            with: WpNetworkResponse(
                body: Data(contentsOf: wpJsonResponse),
                statusCode: 200,
                headerMap: .fromMap(hashMap: ["Link": #"<https://example.com/wp-json/>; rel="https://api.w.org/""#])
            )
        )

        let client = WordPressLoginClient(requestExecutor: stubs)
        let rejectedURL = try XCTUnwrap(URL(string: "x-wordpress-app://login-callback?success=false"))
        do {
            let result = await client.login(
                site: "https://example.com",
                appName: "foo",
                appId: "bar",
                authenticator: Authenticator().returning(.success(rejectedURL))
            )
            let success = try result.get()
            XCTFail("Unexpected successful result: \(success)")
        } catch WordPressLoginClient.Error.authenticationError(.UnsuccessfulLogin) {
            // Do nothing
        } catch {
            XCTFail("Unexpected error: \(error)")
        }
    }

    func testApprovedResult() async throws {
        try stubs.stub(
            url: "https://example.com/",
            with: WpNetworkResponse(
                body: Data(),
                statusCode: 200,
                headerMap: .fromMap(hashMap: ["Link": #"<https://example.com/wp-json/>; rel="https://api.w.org/""#])
            )
        )

        let wpJsonResponse = try XCTUnwrap(
            Bundle.module.url(
                forResource: "Responses/LoginTests-wp-json",
                withExtension: "json"
            )
        )

        try stubs.stub(
            url: "https://example.com/wp-json/",
            with: WpNetworkResponse(
                body: Data(contentsOf: wpJsonResponse),
                statusCode: 200,
                headerMap: .fromMap(hashMap: ["Link": #"<https://example.com/wp-json/>; rel="https://api.w.org/""#])
            )
        )

        let client = WordPressLoginClient(requestExecutor: stubs)
        // swiftlint:disable:next line_length
        let successfulURL = try XCTUnwrap(URL(string: "x-wordpress-app://login-callback?site_url=https%3A%2F%2Fexample.com&user_login=admin&password=123456"))

        let result = await client.login(
            site: "https://example.com",
            appName: "foo",
            appId: "bar",
            authenticator: Authenticator().returning(.success(successfulURL))
        )
        let success = try result.get()
        XCTAssertEqual(success.siteUrl, "https://example.com")
        XCTAssertEqual(success.userLogin, "admin")
        XCTAssertEqual(success.password, "123456")
    }

}

private class Authenticator: WordPressLoginClient.Authenticator {
    var result: Result<URL, WordPressLoginClient.Error>?

    func returning(_ result: Result<URL, WordPressLoginClient.Error>) -> Self {
        self.result = result
        return self
    }

    func authenticate(url: URL, callbackURL: URL) async -> Result<URL, WordPressLoginClient.Error> {
        result ?? .failure(.unknown(NSError(domain: "authenticator-test", code: 1)))
    }
}
