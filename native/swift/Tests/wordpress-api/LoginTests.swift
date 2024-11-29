import Foundation
import Testing

@testable import WordPressAPI

@Suite("Login")
class LoginTests {

    // swiftlint:disable:next force_try
    let appId = { try! WpUuid.parse(input: "caa8b54a-eb5e-4134-8ae2-a3946a428ec7") }()

    @Test
    func testInvalidUrl() async {
        let stubs = HTTPStubs(stubs: [], missingStub: .failure(URLError(.timedOut)))

        let client = WordPressLoginClient(requestExecutor: stubs)

        await #expect(performing: {
            _ = try await client.login(
                site: "invalid url",
                appName: "foo",
                appId: appId,
                authenticator: MockAuthenticator
                    .returning(.failure(.invalidSiteAddress(.UrlDiscoveryFailed(attempts: .init()))))
            )
        }, throws: { error in
            guard let loginError = error as? WordPressLoginClient.Error, case .invalidSiteAddress = loginError else {
                return false
            }

            return true
        })
    }

    @Test
    func testNotWordPressSite() async throws {
        let stubs = HTTPStubs(stubs: [
            HTTPStubs.stub(
                host: "example.com",
                with: WpNetworkResponse(body: Data(), statusCode: 200, headerMap: .empty)
            )
        ], missingStub: .failure(URLError(.timedOut)))

        let client = WordPressLoginClient(requestExecutor: stubs)

        await #expect(
performing: {
            _ = try await client.login(
                site: "https://example.com/blog",
                appName: "foo",
                appId: appId,
                authenticator: MockAuthenticator
                    .returning(
                        .failure(WordPressLoginClient.Error.invalidSiteAddress(.UrlDiscoveryFailed(attempts: .init())))
                    )
            )
        },
 throws: { error in
            guard let loginError = error as? WordPressLoginClient.Error,
 case .invalidSiteAddress = loginError else {
                return false
            }

            #expect(loginError.isAutodiscoveryError())
            #expect(loginError.isFailedToFetchApiRoot, "Error must be a `fetchApiRootUrlError` error")

            return true
        })
    }

    @Test
    func testWpJsonError() async throws {
        let stubs = HTTPStubs(stubs: [
            HTTPStubs.stub(
                url: "https://example.com/",
                with: WpNetworkResponse(
                    body: Data(),
                    statusCode: 200,
                    headerMap: .withLinkHeader(#"<https://example.com/wp-json/>; rel="https://api.w.org/""#)
                )
            ),
            HTTPStubs.stub(
                url: "https://example.com/wp-json/",
                with: WpNetworkResponse(
                    body: "not a json".data(using: .utf8)!,
                    statusCode: 200,
                    headerMap: .withLinkHeader(#"<https://example.com/wp-json/>; rel="https://api.w.org/""#)
                )
            )
        ])

        let client = WordPressLoginClient(requestExecutor: stubs)

        await #expect(performing: {
            _ = try await client.login(
                site: "https://example.com",
                appName: "foo",
                appId: appId,
                authenticator: MockAuthenticator
                    .returning(.failure(WordPressLoginClient.Error.unknown(CocoaError(.fileReadInvalidFileName))))
            )
        }, throws: { error in
            guard let loginError = error as? WordPressLoginClient.Error else {
                return false
            }

            #expect(loginError.isAutodiscoveryError())
            #expect(loginError.isFailedToFetchApiDetails, "Error must be a `fetchApiRootUrlFailed` error")
            return true
        })
    }

    func testMissingAuthenticationEndpoint() async throws {
        let filePath = Bundle.module.url(
            forResource: "Responses/LoginTests-wp-json-missing-authentication-endpoint",
            withExtension: "json"
        )!

        let stubs = HTTPStubs(stubs: [
            HTTPStubs.stub(
                url: "https://example.com/",
                with: WpNetworkResponse(
                    body: Data(),
                    statusCode: 200,
                    headerMap: .withLinkHeader(#"<https://example.com/wp-json/>; rel="https://api.w.org/""#)
                )
            ),
            HTTPStubs.stub(
                url: "https://example.com/wp-json/",
                with: WpNetworkResponse(
                    body: try Data(contentsOf: filePath),
                    statusCode: 200,
                    headerMap: .withLinkHeader(#"<https://example.com/wp-json/>; rel="https://api.w.org/""#)
                )
            )
        ])

        let client = WordPressLoginClient(requestExecutor: stubs)

        await #expect(performing: {
            _ = try await client.login(
                site: "https://example.com",
                appName: "foo",
                appId: appId,
                authenticator: MockAuthenticator.returning(.failure(.missingLoginUrl))
            )
        }, throws: { error in
            error as? WordPressLoginClient.Error == .missingLoginUrl
        })
    }

    func testRejectedResult() async throws {
        let filePath = Bundle.module.url(
            forResource: "Responses/LoginTests-wp-json",
            withExtension: "json"
        )!

        let stubs = HTTPStubs(stubs: [
            HTTPStubs.stub(
                url: "https://example.com/",
                with: WpNetworkResponse(
                    body: Data(),
                    statusCode: 200,
                    headerMap: .withLinkHeader(#"<https://example.com/wp-json/>; rel="https://api.w.org/""#)
                )
            ),
            HTTPStubs.stub(
                url: "https://example.com/wp-json/",
                with: WpNetworkResponse(
                    body: try Data(contentsOf: filePath),
                    statusCode: 200,
                    headerMap: .withLinkHeader(#"<https://example.com/wp-json/>; rel="https://api.w.org/""#)
                )
            )
        ])

        let client = WordPressLoginClient(requestExecutor: stubs)
        let rejectedURL = URL(string: "x-wordpress-app://login-callback?success=false")!

        await #expect(performing: {
            _ = try await client.login(
                site: "https://example.com",
                appName: "foo",
                appId: appId,
                authenticator: MockAuthenticator.returning(.success(rejectedURL))
            )
        }, throws: { error in
            error as? WordPressLoginClient.Error == .authenticationError(.UnsuccessfulLogin)
        })
    }

    func testApprovedResult() async throws {
        let filePath = Bundle.module.url(
            forResource: "Responses/LoginTests-wp-json",
            withExtension: "json"
        )!

        let stubs = HTTPStubs(stubs: [
            HTTPStubs.stub(
                url: "https://example.com/",
                with: WpNetworkResponse(
                    body: Data(),
                    statusCode: 200,
                    headerMap: .withLinkHeader(#"<https://example.com/wp-json/>; rel="https://api.w.org/""#)
                )
            ),
            HTTPStubs.stub(
                url: "https://example.com/wp-json/",
                with: WpNetworkResponse(
                    body: try Data(contentsOf: filePath),
                    statusCode: 200,
                    headerMap: .withLinkHeader(#"<https://example.com/wp-json/>; rel="https://api.w.org/""#)
                )
            )
        ])

        let client = WordPressLoginClient(requestExecutor: stubs)
        // swiftlint:disable:next line_length
        let successfulURL = URL(string: "x-wordpress-app://login-callback?site_url=https%3A%2F%2Fexample.com&user_login=admin&password=123456")!

        let success = try await client.login(
            site: "https://example.com",
            appName: "foo",
            appId: appId,
            authenticator: MockAuthenticator.returning(.success(successfulURL))
        )

        #expect(success.siteUrl == "https://example.com")
        #expect(success.userLogin == "admin")
        #expect(success.password == "123456")
    }
}

private class MockAuthenticator: WordPressLoginClient.AuthenticatorProtocol {
    var result: Result<URL, WordPressLoginClient.Error>

    init(result: Result<URL, WordPressLoginClient.Error>) {
        self.result = result
    }

    static func returning(_ result: Result<URL, WordPressLoginClient.Error>) -> MockAuthenticator {
        MockAuthenticator(result: result)
    }

    func authenticate(url: URL, callbackURL: URL) async throws -> URL {
        do {
            return try result.get()
        } catch {
            throw error
        }
    }
}
