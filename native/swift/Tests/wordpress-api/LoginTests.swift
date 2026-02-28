import Foundation
import Testing

#if canImport(MockWebServer)
import MockWebServer
#endif

#if canImport(Security)
import Security
#endif

@testable import WordPressAPI

#if os(Linux)
import FoundationNetworking
#endif

// swiftlint:disable line_length
@Suite("Login Tests", .enabled(if: !isLinux()))
class LoginTests {

    let client = WordPressLoginClient(urlSession: .init(configuration: .ephemeral))

    @Test("Login Spec Example 1: Valid URL")
    func testValidURL() async throws {
        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(url: "https://vanilla.wpmt.co/", with: .withApiRoot("https://vanilla.wpmt.co/wp-json/")),
            try HTTPStubs.stub(url: "https://vanilla.wpmt.co/wp-json/", with: .loginMockResponse(named: "vanilla-api-root"))
        ])
        let client = WordPressLoginClient(requestExecutor: stubs)
        let parsedUrl = try await client.findLoginUrl(forSite: "https://vanilla.wpmt.co")
        #expect("https://vanilla.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 2: Local Development Environment")
    func testLocalDevelopmentEnvironment() async throws {

        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(url: "http://localhost/", with: .withApiRoot("http://localhost/wp-json")),
            try HTTPStubs.stub(url: "http://localhost/wp-json", with: .jsonResponse(named: "localhost-json-root")),
            try HTTPStubs.stub(url: "https://localhost/", with: .withApiRoot("http://localhost/wp-json")),
            try HTTPStubs.stub(url: "https://localhost/wp-json", with: .jsonResponse(named: "localhost-json-root"))
        ])

        let client = WordPressLoginClient(requestExecutor: stubs)

        await #expect(performing: {
            _ = try await client.findLoginUrl(forSite: "http://localhost")
        }, throws: { error in
            let reason = try #require(try self.getApplicationPasswordsNotSupportedReason(from: error))

            guard case .siteIsLocalDevelopmentEnvironment = reason else {
                Issue.record("The reason should be .SiteIsLocalDevelopmentEnvironment")
                return false
            }

            return true
        })
    }

    @Test("Login Spec Example 3: Admin URL Provided", arguments: [
        ("https://vanilla.wpmt.co/wp-login.php", "https://vanilla.wpmt.co/wp-admin/authorize-application.php"),
        ("https://vanilla.wpmt.co/wp-admin", "https://vanilla.wpmt.co/wp-admin/authorize-application.php")
    ])
    func testAdminUrlProvided(_ provided: String, _ expected: String) async throws {
        // The UserInput attempt uses the admin URL as-is (no stub found -> fails).
        // The AutoStrippedHttps attempt strips the admin suffix and succeeds.
        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(url: "https://vanilla.wpmt.co/", with: .withApiRoot("https://vanilla.wpmt.co/wp-json/")),
            try HTTPStubs.stub(url: "https://vanilla.wpmt.co/wp-json/", with: .loginMockResponse(named: "vanilla-api-root"))
        ])
        let client = WordPressLoginClient(requestExecutor: stubs)
        let parsedUrl = try await client.findLoginUrl(forSite: provided)
        #expect(expected == parsedUrl.url())
    }

    @Test("Login Spec Example 4: HTTP URL with HTTPS Support")
    func testAutoHttpsSupport() async throws {
        // UserInput attempt fetches http://vanilla.wpmt.co/ (no stub -> fails).
        // AutoStrippedHttps attempt converts to https:// and succeeds.
        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(url: "https://vanilla.wpmt.co/", with: .withApiRoot("https://vanilla.wpmt.co/wp-json/")),
            try HTTPStubs.stub(url: "https://vanilla.wpmt.co/wp-json/", with: .loginMockResponse(named: "vanilla-api-root"))
        ])
        let client = WordPressLoginClient(requestExecutor: stubs)
        let parsedUrl = try await client.findLoginUrl(forSite: "http://vanilla.wpmt.co")
        #expect("https://vanilla.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 5: HTTP-only site")
    func testHttpOnlySite() async {
        let stubs: HTTPStubs
        do {
            stubs = HTTPStubs(stubs: [
                try HTTPStubs.stub(url: "http://no-https.wpmt.co/", with: .withApiRoot("http://no-https.wpmt.co/wp-json/")),
                try HTTPStubs.stub(url: "http://no-https.wpmt.co/wp-json/", with: .loginMockResponse(named: "http-only-api-root"))
            ])
        } catch {
            Issue.record("Failed to create stubs: \(error)")
            return
        }
        let client = WordPressLoginClient(requestExecutor: stubs)

        await #expect(performing: {
            _ = try await client.findLoginUrl(forSite: "http://no-https.wpmt.co")
        }, throws: { error in
            let reason = try #require(try self.getApplicationPasswordsNotSupportedReason(from: error))

            guard case .applicationPasswordsDisabledForHttpSite = reason else {
                Issue.record("The reason should be .ApplicationPasswordsDisabledForHttpSite")
                return false
            }

            return true
        })
    }

    @Test("Login Spec Example 6: HTTP-Only Site with Application Password Override")
    func testHttpOnlySiteWithApplicationPasswordsEnabled() async throws {
        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(url: "http://no-https-with-application-passwords.wpmt.co/", with: .withApiRoot("http://no-https-with-application-passwords.wpmt.co/wp-json/")),
            try HTTPStubs.stub(url: "http://no-https-with-application-passwords.wpmt.co/wp-json/", with: .loginMockResponse(named: "http-only-with-app-passwords-api-root"))
        ])
        let client = WordPressLoginClient(requestExecutor: stubs)
        let parsedUrl = try await client.findLoginUrl(forSite: "http://no-https-with-application-passwords.wpmt.co")
        #expect("http://no-https-with-application-passwords.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 7: CDN-Cached Site")
    func testAggressivelyCachedSiteWithNoLinkheader() async throws {
        // Homepage has no Link header, but HTML contains a <link> tag pointing to the API root
        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(url: "https://aggressive-caching.wpmt.co/", with: .htmlResponse(named: "homepage-with-link-tag")),
            try HTTPStubs.stub(url: "https://aggressive-caching.wpmt.co/wp-json/", with: .loginMockResponse(named: "aggressive-caching-api-root"))
        ])
        let client = WordPressLoginClient(requestExecutor: stubs)
        let parsedUrl = try await client.findLoginUrl(forSite: "https://aggressive-caching.wpmt.co")
        #expect("https://aggressive-caching.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 8: Site with Application Passwords Disabled by WordFence")
    func testSiteWithApplicationPasswordsDisabledByWordFence() async throws {
        await #expect(performing: {
            _ = try await self.client.findLoginUrl(forSite: "https://wordfence.wpmt.co")
        }, throws: { error in
            let reason = try #require(try self.getApplicationPasswordsNotSupportedReason(from: error))

            guard case .applicationPasswordBlockedByPlugin(plugin: let plugin) = reason else {
                Issue.record("The reason should be .ApplicationPasswordsDisabledForHttpSite")
                return false
            }

            #expect(plugin.name == "Wordfence")

            return true
        })
    }

    @Test("Login Spec Example 9: Not a WordPress Site", arguments: [
        "google.com",
        "https://google.com"
    ])
    func testNotWordPressSite(url: String) async throws {
        // Homepage returns non-WordPress HTML, no Link header, and no WP markers
        let stubs: HTTPStubs
        do {
            stubs = HTTPStubs(stubs: [
                try HTTPStubs.stub(url: "https://google.com/", with: .htmlResponse(named: "homepage-not-wordpress"))
            ])
        } catch {
            Issue.record("Failed to create stubs: \(error)")
            return
        }
        let client = WordPressLoginClient(requestExecutor: stubs)

        await #expect(performing: {
            _ = try await client.findLoginUrl(forSite: url)
        }, throws: { error in
            try #require(error is AutoDiscoveryAttemptFailure)

            guard let failure = try self.getFindApiRootFailure(from: error) else {
                return false
            }

            if case .probablyNotAWordPressSite = failure {
                return true
            } else {
                return false
            }
        })
    }

    @Test("Login Spec Example 10: WordPress in a subdirectory with a link header")
    func testWordPressSubdirectoryWithLinkHeader() async throws {
        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(url: "https://subdirectory.wpmt.co/index.php?link_header=true", with: .withApiRoot("https://subdirectory.wpmt.co/wordpress/wp-json/")),
            try HTTPStubs.stub(url: "https://subdirectory.wpmt.co/wordpress/wp-json/", with: .loginMockResponse(named: "subdirectory-api-root"))
        ])
        let client = WordPressLoginClient(requestExecutor: stubs)
        let parsedUrl = try await client.findLoginUrl(forSite: "https://subdirectory.wpmt.co/index.php?link_header=true")
        #expect("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 11: WordPress in a subdirectory with a link tag")
    func testWordPressSubdirectoryWithLinkTag() async throws {
        // Homepage has no Link header but HTML contains a <link> tag pointing to subdirectory wp-json
        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(url: "https://subdirectory.wpmt.co/index.php?link_tag=true", with: .htmlResponse(named: "homepage-with-subdirectory-link-tag")),
            try HTTPStubs.stub(url: "https://subdirectory.wpmt.co/wordpress/wp-json/", with: .loginMockResponse(named: "subdirectory-api-root"))
        ])
        let client = WordPressLoginClient(requestExecutor: stubs)
        let parsedUrl = try await client.findLoginUrl(forSite: "https://subdirectory.wpmt.co/index.php?link_tag=true")
        #expect("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 12: WordPress in a subdirectory with a redirect")
    func testWordPressSubdirectory() async throws {
        // In the real scenario, the server redirects to /wordpress/ which has the Link header.
        // With mocks, we simulate the final response directly on the requested URL.
        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(url: "https://subdirectory.wpmt.co/index.php?redirect=true", with: .withApiRoot("https://subdirectory.wpmt.co/wordpress/wp-json/")),
            try HTTPStubs.stub(url: "https://subdirectory.wpmt.co/wordpress/wp-json/", with: .loginMockResponse(named: "subdirectory-api-root"))
        ])
        let client = WordPressLoginClient(requestExecutor: stubs)
        let parsedUrl = try await client.findLoginUrl(forSite: "https://subdirectory.wpmt.co/index.php?redirect=true")
        #expect("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 13: Site uses HTTP basic with no provided credentials")
    func testWordPressHttpBasic() async throws {
        let stubs: HTTPStubs
        do {
            stubs = HTTPStubs(stubs: [
                try HTTPStubs.stub(host: "basic-auth.wpmt.co", with: .responseWithStatus(401, headers: ["WWW-Authenticate": "Basic realm=\"Restricted\""]))
            ])
        } catch {
            Issue.record("Failed to create stubs: \(error)")
            return
        }
        let client = WordPressLoginClient(requestExecutor: stubs)

        await #expect(performing: {
            _ = try await client.findLoginUrl(forSite: "https://basic-auth.wpmt.co")
        }, throws: { error in
            let reason = try #require(try self.getRequestExecutionErrorReason(from: error))

            guard case .httpAuthenticationRequiredError(hostname: let hostname, method: _) = reason else {
                Issue.record("The transport error must be `httpAuthenticationRequiredError`")
                return false
            }

            if hostname == "https://basic-auth.wpmt.co" {
                Issue.record("Hostname shouldn't contain the scheme: \(hostname)")
            }

            return true
        })
    }

    @Test("Login Spec Example 13: Site uses HTTP basic with invalid credentials provided")
    func testWordPressHttpBasicWithInvalidCredentials() async throws {
        let stubs: HTTPStubs
        do {
            stubs = HTTPStubs(stubs: [
                try HTTPStubs.stub(host: "basic-auth.wpmt.co", with: .responseWithStatus(401, headers: ["WWW-Authenticate": "Basic realm=\"Restricted\""]))
            ])
        } catch {
            Issue.record("Failed to create stubs: \(error)")
            return
        }
        let invalid = ApiDiscoveryAuthenticationMiddleware(username: "invalid", password: "invalid")

        await #expect(performing: {
            _ = try await WordPressLoginClient(
                requestExecutor: stubs,
                middleware: MiddlewarePipeline(middlewares: invalid)
            ).findLoginUrl(forSite: "https://basic-auth.wpmt.co")
        }, throws: { error in
            let reason = try #require(try self.getRequestExecutionErrorReason(from: error))

            guard case .httpAuthenticationRejectedError(hostname: let hostname, method: _) = reason else {
                Issue.record("The transport error must be `httpAuthenticationRequiredError`")
                return false
            }

            if hostname == "https://basic-auth.wpmt.co" {
                Issue.record("Hostname shouldn't contain the scheme: \(hostname)")
            }

            return true
        })
    }

    @Test("Login Spec Example 13: Site uses HTTP basic with correct credentials provided")
    func testWordPressHttpBasicWithValidCredentials() async throws {
        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(url: "https://basic-auth.wpmt.co/", with: .withApiRoot("https://basic-auth.wpmt.co/wp-json/")),
            try HTTPStubs.stub(url: "https://basic-auth.wpmt.co/wp-json/", with: .loginMockResponse(named: "basic-auth-api-root"))
        ])
        let valid = ApiDiscoveryAuthenticationMiddleware(username: "test@example.com", password: "str0ngp4ssw0rd!")

        let parsedUrl = try await WordPressLoginClient(
            requestExecutor: stubs,
            middleware: MiddlewarePipeline(middlewares: valid)
        ).findLoginUrl(forSite: "https://basic-auth.wpmt.co")

        #expect("https://basic-auth.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 14: Custom REST API Prefix")
    func testWordPressCustomRestApiPrefix() async throws {
        // Site uses a custom REST prefix (e.g., /custom-api/ instead of /wp-json/)
        // The Link header points to the custom API root
        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(url: "https://custom-rest-prefix.wpmt.co/", with: .withApiRoot("https://custom-rest-prefix.wpmt.co/custom-api/")),
            try HTTPStubs.stub(url: "https://custom-rest-prefix.wpmt.co/custom-api/", with: .loginMockResponse(named: "custom-rest-prefix-api-root"))
        ])
        let client = WordPressLoginClient(requestExecutor: stubs)
        let parsedUrl = try await client.findLoginUrl(forSite: "https://custom-rest-prefix.wpmt.co")
        #expect("https://custom-rest-prefix.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 15: Rate Limited")
    func testWordPressHeavyRateLimiting() async throws {
        let parsedUrl = try await client.findLoginUrl(forSite: "https://aggressive-rate-limiting.wpmt.co")
        #expect("https://aggressive-rate-limiting.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 15: Rate Limited that never succeeds")
    func testWordPressHeavyRateLimitingThatNeverSucceeds() async throws {
        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(host: "aggressive-rate-limiting.wpmt.co", with: .retryResponse(after: 1))
        ])

        await #expect(performing: {
            let retryMiddleware = RetryAfterMiddleware(maxRetries: 3, maxRetryWaitSeconds: 1)
            let client = WordPressLoginClient(requestExecutor: stubs, middleware: MiddlewarePipeline(middlewares: [retryMiddleware]))
            _ = try await client.findLoginUrl(forSite: "https://aggressive-rate-limiting.wpmt.co")
        }, throws: { error in
            let reason = try #require(try self.getRequestExecutionErrorReason(from: error))

            guard case .misconfiguredRateLimitError = reason else {
                Issue.record("The transport error must be `misconfiguredRateLimitError`")
                return false
            }

            return true
        })
    }

    @Test("Login Spec Example 16: Non-existent website")
    func testInvalidUrl() async {
        await #expect(performing: {
            _ = try await self.client.findLoginUrl(forSite: "https://valid-looking-url-but-not-actually.foo")
        }, throws: { error in
            let reason = try #require(try self.getRequestExecutionErrorReason(from: error))

            guard case .nonExistentSiteError = reason else {
                Issue.record("The transport error must be `nonExistentSiteError`")
                return false
            }

            return true
        })
    }

    #if canImport(MockWebServer)
    @Test("Login Spec Example 17: Invalid SSL Certificate")
    func testInvalidHTTPsFails() async throws {
        let server = MockWebServer()
        try server.start(tls: .wrongHostname())
        defer { server.shutdown() }

        server.enqueue(MockResponse(statusCode: 200))

        let siteUrl = server.url(forPath: "/").absoluteString
        let client = WordPressLoginClient(urlSession: .init(configuration: .ephemeral))

        await #expect(performing: {
            _ = try await client.findLoginUrl(forSite: siteUrl)
        }, throws: { error in
            let reason = try #require(try self.getRequestExecutionErrorReason(from: error))

            guard case .invalidSslError(let underlyingReason) = reason else {
                Issue.record("The transport error must be `invalidSslError`")
                return false
            }

            #if os(watchOS) // watchOS doesn't make the underlying certificate available to us
            guard case .genericSslError = underlyingReason else {
                Issue.record("The underlying error must be `genericSslError`")
                return false
            }
            #else
            guard case .certificateNotValidForName(let hostname, let presentedHostnames) = underlyingReason else {
                Issue.record("The underlying error must be `certificateNotValidForName`")
                return false
            }

            #expect(hostname == "127.0.0.1")
            #expect(presentedHostnames == ["wrong.example.com"])
            #endif

            return true
        })
    }

    /// This test is unavailable in Linux until https://github.com/swiftlang/swift-corelibs-foundation/pull/4937 lands
    @Test("Login Spec Example 18: Invalid SSL Certificate with explicit exception", .enabled(if: !isLinux()))
    func testInvalidHttpsWithExceptionWorks() async throws {
        let server = MockWebServer()
        try server.start(tls: .wrongHostname())
        defer { server.shutdown() }

        let port = server.port
        let baseUrl = "https://127.0.0.1:\(port)"

        server.route("/", MockResponse(statusCode: 200)
            .withHeader("Link", "<\(baseUrl)/wp-json/>; rel=\"https://api.w.org/\""))

        server.route("/wp-json/", .json("""
        {"name":"Test Site","description":"","url":"\(baseUrl)","home":"\(baseUrl)","gmt_offset":0,"timezone_string":"UTC","namespaces":["oembed/1.0","wp/v2","wp-site-health/v1"],"authentication":{"application-passwords":{"endpoints":{"authorization":"\(baseUrl)/wp-admin/authorize-application.php"}}},"routes":{},"site_logo":0,"site_icon":0,"site_icon_url":""}
        """))

        let executor = WpRequestExecutor(urlSession: .init(configuration: .ephemeral))
        executor.allowSSL(altNames: ["127.0.0.1"], forCommonName: "wrong.example.com")
        let client = WordPressLoginClient(requestExecutor: executor)
        _ = try await client.findLoginUrl(forSite: baseUrl)
    }

    /// This test requires the test CA to be trusted: `make trust-test-ca`
    @Test("Login Spec Example 19: Alternative name in SSL Certificate", .enabled(if: !isLinux() && isTestCATrusted()))
    func testAlternativeNameWorks() async throws {
        guard let p12Url = Bundle.module.url(forResource: "san-test", withExtension: "p12", subdirectory: "ssl-certs") else {
            preconditionFailure("Could not find san-test.p12 in ssl-certs")
        }
        let p12Data = try Data(contentsOf: p12Url)

        let server = MockWebServer()
        try server.start(tls: TLSConfiguration(p12Data: p12Data, password: "test"))
        defer { server.shutdown() }

        let port = server.port
        let baseUrl = "https://127.0.0.1:\(port)"

        server.route("/", MockResponse(statusCode: 200)
            .withHeader("Link", "<\(baseUrl)/wp-json/>; rel=\"https://api.w.org/\""))

        server.route("/wp-json/", .json("""
        {"name":"Test Site","description":"","url":"\(baseUrl)","home":"\(baseUrl)","gmt_offset":0,"timezone_string":"UTC","namespaces":["oembed/1.0","wp/v2","wp-site-health/v1"],"authentication":{"application-passwords":{"endpoints":{"authorization":"\(baseUrl)/wp-admin/authorize-application.php"}}},"routes":{},"site_logo":0,"site_icon":0,"site_icon_url":""}
        """))

        // Connect using default URLSession (no allowSSL bypass) — succeeds because
        // 127.0.0.1 is a SAN on the cert and the CA is trusted in the system keychain
        let client = WordPressLoginClient(urlSession: .init(configuration: .ephemeral))
        _ = try await client.findLoginUrl(forSite: baseUrl)
    }
    #endif // canImport(MockWebServer)

    @Test("Cancel API discovery process")
    func testCancellation() async throws {
        let task = Task { [client] in
            let success = try await client.details(ofSite: "https://vanilla.wpmt.co")
            Issue.record("The function should throw. \(success)")
        }

        await #expect(
            performing: {
                try await Task.sleep(for: .milliseconds(800))
                task.cancel()

                try await task.value
            },
            throws: { error in
                error is AutoDiscoveryAttemptFailure || error is CancellationError
            },
        )
    }

    private func getApplicationPasswordsNotSupportedReason(
        from error: any Error
    ) throws -> ApplicationPasswordsNotSupportedReason? {
        try #require(error is AutoDiscoveryAttemptFailure)

        if let error = try getFetchAndParseApiRootFailure(from: error) {
            if case .applicationPasswordsNotSupported(_, let reason) = error {
                return reason
            }
        }

        return nil
    }

    private func getFindApiRootFailure(from error: any Error) throws -> FindApiRootFailure? {
        try #require(error is AutoDiscoveryAttemptFailure)

        if case .FindApiRoot(_, let failure) = error as? AutoDiscoveryAttemptFailure {
            return failure
        }

        return nil
    }

    private func getFetchAndParseApiRootFailure(from error: any Error) throws -> FetchAndParseApiRootFailure? {
        try #require(error is AutoDiscoveryAttemptFailure)

        if case .FetchAndParseApiRoot(parsedSiteUrl: _, apiRootUrl: _, let failure) = error as? AutoDiscoveryAttemptFailure {
            return failure
        }

        return nil
    }

    private func getRequestExecutionErrorReason(from error: any Error) throws -> RequestExecutionErrorReason? {
        try #require(error is AutoDiscoveryAttemptFailure)

        if let failure = try getFindApiRootFailure(from: error) {
            if case .fetchHomepage(let transportError) = failure {
                if case .RequestExecutionFailed(_, _, let reason) = transportError {
                    return reason
                }
            }
        }

        if let failure = try getFetchAndParseApiRootFailure(from: error) {
            if case .fetchApiRoot(let transportError) = failure {
                if case .RequestExecutionFailed(_, _, let reason) = transportError {
                    return reason
                }
            }
        }

        Issue.record("Failed to find a request execution error reason")

        return nil
    }
}

// swiftlint:enable line_length
