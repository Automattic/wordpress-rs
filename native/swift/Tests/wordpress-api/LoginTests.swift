import Foundation
import Testing

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
        let parsedUrl = try await findLoginUrl(forSite: "https://vanilla.wpmt.co")
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

        await #expect(
            performing: {
                _ = try await client.details(ofSite: "http://localhost")
            },
            throws: { error in
                let reason = try #require(try self.getApplicationPasswordsNotSupportedReason(from: error))

                guard case .siteIsLocalDevelopmentEnvironment = reason else {
                    Issue.record("The reason should be .SiteIsLocalDevelopmentEnvironment")
                    return false
                }

                return true
            }
        )
    }

    @Test(
        "Login Spec Example 3: Admin URL Provided",
        arguments: [
            ("https://vanilla.wpmt.co/wp-login.php", "https://vanilla.wpmt.co/wp-admin/authorize-application.php"),
            ("https://vanilla.wpmt.co/wp-admin", "https://vanilla.wpmt.co/wp-admin/authorize-application.php")
        ]
    )
    func testAdminUrlProvided(_ provided: String, _ expected: String) async throws {
        let parsedUrl = try await findLoginUrl(forSite: provided)
        #expect(expected == parsedUrl.url())
    }

    @Test("Login Spec Example 4: HTTP URL with HTTPS Support")
    func testAutoHttpsSupport() async throws {
        let parsedUrl = try await findLoginUrl(forSite: "http://vanilla.wpmt.co")
        #expect("https://vanilla.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 5: HTTP-only site")
    func testHttpOnlySite() async {
        await #expect(
            performing: {
                _ = try await self.client.details(ofSite: "http://no-https.wpmt.co")
            },
            throws: { error in
                let reason = try #require(try self.getApplicationPasswordsNotSupportedReason(from: error))

                guard case .applicationPasswordsDisabledForHttpSite = reason else {
                    Issue.record("The reason should be .ApplicationPasswordsDisabledForHttpSite")
                    return false
                }

                return true
            }
        )
    }

    @Test("Login Spec Example 6: HTTP-Only Site with Application Password Override")
    func testHttpOnlySiteWithApplicationPasswordsEnabled() async throws {
        let parsedUrl = try await findLoginUrl(forSite: "http://no-https-with-application-passwords.wpmt.co")
        #expect(
            "http://no-https-with-application-passwords.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url()
        )
    }

    @Test("Login Spec Example 7: CDN-Cached Site")
    func testAggressivelyCachedSiteWithNoLinkheader() async throws {
        let parsedUrl = try await findLoginUrl(forSite: "https://aggressive-caching.wpmt.co")
        #expect("https://aggressive-caching.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 8: Site with Application Passwords Disabled by WordFence")
    func testSiteWithApplicationPasswordsDisabledByWordFence() async throws {
        await #expect(
            performing: {
                _ = try await self.client.details(ofSite: "https://wordfence.wpmt.co")
            },
            throws: { error in
                let reason = try #require(try self.getApplicationPasswordsNotSupportedReason(from: error))

                guard case .applicationPasswordBlockedByPlugin(plugin: let plugin) = reason else {
                    Issue.record("The reason should be .ApplicationPasswordsDisabledForHttpSite")
                    return false
                }

                #expect(plugin.name == "Wordfence")

                return true
            }
        )
    }

    @Test(
        "Login Spec Example 9: Not a WordPress Site",
        arguments: [
            "google.com",
            "https://google.com"
        ]
    )
    func testNotWordPressSite(url: String) async throws {
        await #expect(
            performing: {
                _ = try await self.client.details(ofSite: url)
            },
            throws: { error in
                try #require(error is AutoDiscoveryAttemptFailure)

                guard let failure = try self.getFindApiRootFailure(from: error) else {
                    return false
                }

                if case .probablyNotAWordPressSite = failure {
                    return true
                } else {
                    return false
                }
            }
        )
    }

    @Test("Login Spec Example 10: WordPress in a subdirectory with a link header")
    func testWordPressSubdirectoryWithLinkHeader() async throws {
        let parsedUrl = try await findLoginUrl(forSite: "https://subdirectory.wpmt.co/index.php?link_header=true")
        #expect("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 11: WordPress in a subdirectory with a link tag")
    func testWordPressSubdirectoryWithLinkTag() async throws {
        let parsedUrl = try await findLoginUrl(forSite: "https://subdirectory.wpmt.co/index.php?link_tag=true")
        #expect("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 12: WordPress in a subdirectory with a redirect")
    func testWordPressSubdirectory() async throws {
        let parsedUrl = try await findLoginUrl(forSite: "https://subdirectory.wpmt.co/index.php?redirect=true")
        #expect("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 13: Site uses HTTP basic with no provided credentials")
    func testWordPressHttpBasic() async throws {
        await #expect(
            performing: {
                _ = try await self.client.details(ofSite: "https://basic-auth.wpmt.co")
            },
            throws: { error in
                let reason = try #require(try self.getRequestExecutionErrorReason(from: error))

                guard case .httpAuthenticationRequiredError(hostname: let hostname, method: _) = reason else {
                    Issue.record("The transport error must be `httpAuthenticationRequiredError`")
                    return false
                }

                if hostname == "https://basic-auth.wpmt.co" {
                    Issue.record("Hostname shouldn't contain the scheme: \(hostname)")
                }

                return true
            }
        )
    }

    @Test("Login Spec Example 13: Site uses HTTP basic with invalid credentials provided")
    func testWordPressHttpBasicWithInvalidCredentials() async throws {
        let invalid = ApiDiscoveryAuthenticationMiddleware(username: "invalid", password: "invalid")

        await #expect(
            performing: {
                _ = try await WordPressLoginClient(
                    urlSession: .init(configuration: .ephemeral),
                    middleware: MiddlewarePipeline(middlewares: invalid)
                )
                .details(ofSite: "https://basic-auth.wpmt.co")
            },
            throws: { error in
                let reason = try #require(try self.getRequestExecutionErrorReason(from: error))

                guard case .httpAuthenticationRejectedError(hostname: let hostname, method: _) = reason else {
                    Issue.record("The transport error must be `httpAuthenticationRequiredError`")
                    return false
                }

                if hostname == "https://basic-auth.wpmt.co" {
                    Issue.record("Hostname shouldn't contain the scheme: \(hostname)")
                }

                return true
            }
        )
    }

    @Test("Login Spec Example 13: Site uses HTTP basic with correct credentials provided")
    func testWordPressHttpBasicWithValidCredentials() async throws {
        let valid = ApiDiscoveryAuthenticationMiddleware(username: "test@example.com", password: "str0ngp4ssw0rd!")

        let result = try await WordPressLoginClient(
            urlSession: .init(configuration: .ephemeral),
            middleware: MiddlewarePipeline(middlewares: valid)
        )
        .details(ofSite: "https://basic-auth.wpmt.co")

        let parsedUrl = result.authentication.applicationPasswordsUrl!
        #expect("https://basic-auth.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 14: Custom REST API Prefix")
    func testWordPressCustomRestApiPrefix() async throws {
        let parsedUrl = try await findLoginUrl(forSite: "https://custom-rest-prefix.wpmt.co")
        #expect("https://custom-rest-prefix.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 15: Rate Limited")
    func testWordPressHeavyRateLimiting() async throws {
        let parsedUrl = try await findLoginUrl(forSite: "https://aggressive-rate-limiting.wpmt.co")
        #expect("https://aggressive-rate-limiting.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 15: Rate Limited that never succeeds")
    func testWordPressHeavyRateLimitingThatNeverSucceeds() async throws {
        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(host: "aggressive-rate-limiting.wpmt.co", with: .retryResponse(after: 1))
        ])

        await #expect(
            performing: {
                let retryMiddleware = RetryAfterMiddleware(maxRetries: 3, maxRetryWaitSeconds: 1)
                let client = WordPressLoginClient(
                    requestExecutor: stubs,
                    middleware: MiddlewarePipeline(middlewares: [retryMiddleware])
                )
                _ = try await client.details(ofSite: "https://aggressive-rate-limiting.wpmt.co")
            },
            throws: { error in
                let reason = try #require(try self.getRequestExecutionErrorReason(from: error))

                guard case .misconfiguredRateLimitError = reason else {
                    Issue.record("The transport error must be `misconfiguredRateLimitError`")
                    return false
                }

                return true
            }
        )
    }

    @Test("Login Spec Example 16: Non-existent website")
    func testInvalidUrl() async {
        await #expect(
            performing: {
                _ = try await self.client.details(ofSite: "https://valid-looking-url-but-not-actually.foo")
            },
            throws: { error in
                let reason = try #require(try self.getRequestExecutionErrorReason(from: error))

                guard case .nonExistentSiteError = reason else {
                    Issue.record("The transport error must be `nonExistentSiteError`")
                    return false
                }

                return true
            }
        )
    }

    @Test("Login Spec Example 17: Invalid SSL Certificate")
    func testInvalidHTTPsFails() async throws {
        // `wrong.host.badssl.com` serves a valid, trusted certificate for `*.badssl.com`. A
        // single-label wildcard doesn't cover the three-label host, so the chain is fine but the
        // name doesn't match — the pure name-mismatch case.
        await #expect(
            performing: {
                _ = try await self.client.details(ofSite: "https://wrong.host.badssl.com")
            },
            throws: { error in
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

                #expect(hostname == "wrong.host.badssl.com")
                // The presented names are the leaf certificate's Common Name and
                // its SANs, not only the CN. Assert the certificate's identity is
                // reported and the requested host is absent, without pinning the
                // exact SAN list (it changes when the certificate is reissued).
                #expect(presentedHostnames.contains("*.badssl.com"))
                #expect(!presentedHostnames.contains("wrong.host.badssl.com"))
                #endif

                return true
            }
        )
    }

    /// This test is unavailable in Linux until https://github.com/swiftlang/swift-corelibs-foundation/pull/4937 lands
    @Test("Login Spec Example 17: Invalid SSL Certificate with explicit exception", .enabled(if: !isLinux()))
    func testInvalidHttpsWithExceptionWorks() async throws {
        // Allow-list the name-mismatched host for the certificate's common name. The chain is
        // still valid, so `allowAlternativeNames` accepts it and the request gets past the handshake.
        let executor = WpRequestExecutor(urlSession: .init(configuration: .ephemeral))
        executor.allowAlternativeNames(["wrong.host.badssl.com"], forCommonName: "*.badssl.com")
        let client = WordPressLoginClient(requestExecutor: executor)

        await #expect(
            performing: {
                _ = try await client.details(ofSite: "https://wrong.host.badssl.com")
            },
            throws: { error in
                // The certificate is accepted, so discovery gets past the handshake and fails only
                // because badssl.com isn't a WordPress site. Assert that specific failure so the
                // test pins that the request actually reached the host — "not an SSL error" would
                // also hold for a DNS failure, a timeout, or a reset that never touched badssl.com.
                guard let failure = try self.getFindApiRootFailure(from: error),
                    case .probablyNotAWordPressSite = failure
                else {
                    Issue.record("Expected discovery to reach the host and fail as .probablyNotAWordPressSite")
                    return false
                }
                return true
            }
        )
    }

    /// `allowAlternativeNames(_:forCommonName:)` must not become a blanket bypass: a self-signed
    /// certificate is rejected even when its host is allow-listed, because the chain is still
    /// validated. Regression test for https://github.com/Automattic/wordpress-rs/issues/1512.
    @Test("Alternative-name exception still validates the certificate chain", .enabled(if: !isLinux()))
    func testAllowAlternativeNamesStillValidatesChain() async throws {
        let executor = WpRequestExecutor(urlSession: .init(configuration: .ephemeral))
        executor.allowAlternativeNames(["self-signed.badssl.com"], forCommonName: "*.badssl.com")
        let client = WordPressLoginClient(requestExecutor: executor)

        await #expect(
            performing: {
                _ = try await client.details(ofSite: "https://self-signed.badssl.com")
            },
            throws: { error in
                let reason = try #require(try self.getRequestExecutionErrorReason(from: error))
                guard case .invalidSslError = reason else {
                    Issue.record("A self-signed certificate must be rejected as an `invalidSslError`")
                    return false
                }
                return true
            }
        )
    }

    /// `disableCertificateValidation(forHost:)` is the explicit full-bypass counterpart to
    /// `allowAlternativeNames(_:forCommonName:)`: it accepts even a self-signed certificate.
    /// This test is unavailable in Linux until https://github.com/swiftlang/swift-corelibs-foundation/pull/4937 lands
    @Test("Disable certificate validation for a host", .enabled(if: !isLinux()))
    func testDisableCertificateValidationWorks() async throws {
        let executor = WpRequestExecutor(urlSession: .init(configuration: .ephemeral))
        executor.disableCertificateValidation(forHost: "self-signed.badssl.com")
        let client = WordPressLoginClient(requestExecutor: executor)

        await #expect(
            performing: {
                _ = try await client.details(ofSite: "https://self-signed.badssl.com")
            },
            throws: { error in
                // The self-signed certificate is accepted, so discovery reaches the host and fails
                // only because badssl.com isn't a WordPress site — not merely "not an SSL error".
                guard let failure = try self.getFindApiRootFailure(from: error),
                    case .probablyNotAWordPressSite = failure
                else {
                    Issue.record("Expected discovery to reach the host and fail as .probablyNotAWordPressSite")
                    return false
                }
                return true
            }
        )
    }

    /// Regression for a SAN-only (Common-Name-less) leaf certificate — see #1508.
    /// `no-common-name.badssl.com` serves a certificate whose subject carries no
    /// Common Name and a single SAN. Before the fix the leaf failed to parse, the
    /// `compactMap` dropped it, and element 0 of the survivors — the issuer CA —
    /// was reported, so `presentedHostnames` was the CA's name (`COMODO ...`)
    /// rather than the site's.
    @Test("SAN-only certificate reports its SAN, not the issuer CA")
    func testCommonNameLessCertificateReportsSan() async throws {
        await #expect(
            performing: {
                _ = try await self.client.details(ofSite: "https://no-common-name.badssl.com")
            },
            throws: { error in
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
                // Breadcrumb (#1498): this endpoint's certificate is expired, so the failure is a
                // bad-date one that the executor currently reports as `certificateNotValidForName`.
                // Once #1498 remaps bad-date failures to `genericSslError`, this endpoint yields no
                // presented hostnames and the assertion below breaks — move it to a non-expired
                // Common-Name-less certificate then (e.g. a local mock, #1208). The parsing itself
                // is already covered #1498-proof by the Rust `ssl` unit tests.
                guard case .certificateNotValidForName(_, let presentedHostnames) = underlyingReason else {
                    Issue.record("The underlying error must be `certificateNotValidForName`")
                    return false
                }

                // The leaf carries no Common Name and exactly one SAN, so that SAN
                // is the entire presented-hostname list. The bug reported the
                // COMODO issuer CA's name here instead.
                #expect(presentedHostnames == ["no-common-name.badssl.com"])
                #endif

                return true
            }
        )
    }

    /// Regression for the SANs half of the same payload — see #1507.
    /// `wrong.host.badssl.com` serves a valid `*.badssl.com` certificate on a host
    /// it doesn't cover, so it's a genuine name mismatch whose identities live in
    /// the SANs. `presentedHostnames` must include them, not only the Common Name.
    @Test("Name-mismatch certificate reports its SANs, not only its CN")
    func testNameMismatchReportsAllPresentedNames() async throws {
        await #expect(
            performing: {
                _ = try await self.client.details(ofSite: "https://wrong.host.badssl.com")
            },
            throws: { error in
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

                #expect(hostname == "wrong.host.badssl.com")
                // The certificate is for `*.badssl.com` with SANs `*.badssl.com`
                // and `badssl.com`. The old code reported only the CN, so the
                // `badssl.com` SAN is the discriminator that proves SANs are now
                // included.
                #expect(presentedHostnames.contains("badssl.com"))
                #expect(presentedHostnames.contains("*.badssl.com"))
                #endif

                return true
            }
        )
    }

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

    private func findLoginUrl(forSite url: String) async throws -> ParsedUrl {
        let result = try await client.details(ofSite: url)
        return result.authentication.applicationPasswordsUrl!
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

        if case .FetchAndParseApiRoot(parsedSiteUrl: _, apiRootUrl: _, let failure) = error
            as? AutoDiscoveryAttemptFailure
        {
            return failure
        }

        return nil
    }

    private func getRequestExecutionErrorReason(from error: any Error) throws -> RequestExecutionErrorReason? {
        try #require(error is AutoDiscoveryAttemptFailure)

        if let failure = try getFindApiRootFailure(from: error) {
            if case .fetchHomepage(let transportError) = failure {
                if case .RequestExecutionFailed(_, _, let reason, _, _) = transportError {
                    return reason
                }
            }
        }

        if let failure = try getFetchAndParseApiRootFailure(from: error) {
            if case .fetchApiRoot(let transportError) = failure {
                if case .RequestExecutionFailed(_, _, let reason, _, _) = transportError {
                    return reason
                }
            }
        }

        Issue.record("Failed to find a request execution error reason")

        return nil
    }
}

// swiftlint:enable line_length
