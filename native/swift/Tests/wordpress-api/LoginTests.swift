import Foundation
import Testing

@testable import WordPressAPI

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

// swiftlint:disable line_length
@Suite("Parallel Login Test")
class LoginTests {

    let client = WordPressLoginClient(urlSession: .shared)

    @Test("Login Spec Example 1: Valid URL")
    func testValidURL() async throws {
        let parsedUrl = try await client.loginURL(forSite: "https://vanilla.wpmt.co")
        #expect("https://vanilla.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 2: Local Development Environment")
    func testLocalDevelopmentEnvironment() async throws {

        let stubs = HTTPStubs(stubs: [
            try HTTPStubs.stub(url: "http://localhost/", with: .init(body: Data(), statusCode: 200, headerMap: .fromMap(hashMap: [
                "Link": "<http://localhost/wp-json/>; rel=\"https://api.w.org/\""
            ]))),
            try HTTPStubs.stub(url: "http://localhost/wp-json/", with: .jsonResponse(named: "localhost-json-root"))
        ])

        let client = WordPressLoginClient(requestExecutor: stubs)

        await #expect(performing: {
            _ = try await client.loginURL(forSite: "http://localhost")
        }, throws: { error in
            #expect(error is LoginError)
            #expect("This site is a local development environment. You'll need to enable application passwords to connect to it with the app." == error.localizedDescription)
            return true
        })
    }

    @Test("Login Spec Example 3: Admin URL Provided")
    func testAdminUrlProvided() async throws {
        let parsedUrl = try await client.loginURL(forSite: "https://vanilla.wpmt.co/wp-login.php")
        #expect("https://vanilla.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 4: HTTP URL with HTTPS Support")
    func testAutoHttpsSupport() async throws {
        let parsedUrl = try await client.loginURL(forSite: "http://vanilla.wpmt.co")
        #expect("https://vanilla.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 5: HTTP-only site")
    func testHttpOnlySite() async {
        await #expect(performing: {
            _ = try await client.loginURL(forSite: "http://optional-https.wpmt.co")
        }, throws: { error in
            #expect(error is LoginError)
            #expect("Application Passwords is not enabled for this site – this is likely because we can't establish a secure connection to it. Please add an SSL certificate to this site and try again." == (error as? LoginError)?.errorDescription)

            return true
        })
    }

    @Test("Login Spec Example 6: HTTP-Only Site with Application Password Override")
    func testHttpOnlySiteWithApplicationPasswordsEnabled() async throws {
        let parsedUrl = try await client.loginURL(forSite: "http://no-https.wpmt.co")
        #expect("http://no-https.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 7: CDN-Cached Site")
    func testAggressivelyCachedSiteWithNoLinkheader() async throws {
        let parsedUrl = try await client.loginURL(forSite: "https://aggressive-caching.wpmt.co")
        #expect("https://aggressive-caching.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 8: Site with Application Passwords Disabled by WordFence")
    func testSiteWithApplicationPasswordsDisabledByWordFence() async throws {
        await #expect(performing: {
            _ = try await client.loginURL(forSite: "https://wordfence.wpmt.co")
        }, throws: { error in
            #expect(error is LoginError)
            #expect("Unable to login to https://wordfence.wpmt.co – the Wordfence plugin might have disabled Application Passwords. Please visit https://www.wordfence.com/support/ to learn more." == (error as? LoginError)?.errorDescription)
            return true
        })
    }

    @Test("Login Spec Example 9: Not a WordPress Site")
    func testNotWordPressSite() async throws {
        await #expect(performing: {
            _ = try await client.loginURL(forSite: "https://google.com")
        }, throws: { error in
            #expect(error is LoginError)
            #expect("Unable to login to https://google.com. Please double-check that this is a WordPress site" == (error as? LoginError)?.errorDescription)
            return true
        })
    }

    @Test("Login Spec Example 10: WordPress in a subdirectory with a link header")
    func testWordPressSubdirectoryWithLinkHeader() async throws {
        let parsedUrl = try await client.loginURL(forSite: "https://subdirectory.wpmt.co/index.php?link_header=true")
        #expect("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php" == parsedUrl.url())
    }


    @Test("Login Spec Example 13: Site uses HTTP basic with no provided credentials")
    func testWordPressHttpBasic() async throws {
        await #expect(performing: {
            _ = try await client.loginURL(forSite: "https://basic-auth.wpmt.co")
        }, throws: { error in
            #expect(error is LoginError)
            #expect("The server at https://basic-auth.wpmt.co/ requires authentication. Please provide your username and password." == (error as? LoginError)?.errorDescription)
            return true
        })
    }

    @Test("Login Spec Example 13: Site uses HTTP basic with invalid credentials provided")
    func testWordPressHttpBasicWithInvalidCredentials() async throws {
        let credential = URLCredential(user: "invalid", password: "invalid", persistence: .none)

        await #expect(performing: {
            _ = try await client.loginURL(
                forSite: "https://basic-auth.wpmt.co",
                credential: credential
            )
        }, throws: { error in
            #expect(error is LoginError)
            #expect("The server at https://basic-auth.wpmt.co/ rejected your credentials. Please provide a valid username and password." == (error as? LoginError)?.errorDescription)
            return true
        })
    }

    @Test("Login Spec Example 13: Site uses HTTP basic with correct credentials provided")
    func testWordPressHttpBasicWithValidCredentials() async throws {
        let credential = URLCredential(
            user: "test@example.com",
            password: "str0ngp4ssw0rd!",
            persistence: .none
        )

        let parsedUrl = try await client.loginURL(
            forSite: "https://basic-auth.wpmt.co",
            credential: credential
        )

        #expect("https://basic-auth.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 14: Custom REST API Prefix")
    func testWordPressCustomRestApiPrefix() async throws {
        let parsedUrl = try await client.loginURL(forSite: "https://custom-rest-prefix.wpmt.co")
        #expect("https://custom-rest-prefix.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 16: Non-existent website")
    func testInvalidUrl() async {
        await #expect(performing: {
            _ = try await client.loginURL(forSite: "https://valid-looking-url-but-not-actually.foo")
        }, throws: { error in
            #expect(error is LoginError)
            #expect("Unable to login to https://valid-looking-url-but-not-actually.foo. Please double-check that this is a WordPress site" == (error as? LoginError)?.errorDescription)
            return true
        })
    }

    @Test("Linux issue")
    func testLinuxIssue() async {
        do {
            _ = try await URLSession.shared.data(from: URL(string: "https://valid-looking-url-but-not-actually.foo")!)
        } catch {
            #expect(error is URLError)
        }
    }

    @Test("Login Spec Example 17: Invalid SSL Certificate", .enabled(if: !isLinux()))
    func testInvalidHTTPsFails() async throws {
        let session = URLSession(configuration: .default)
        let client = WordPressLoginClient(urlSession: session)
        await #expect(performing: {
            _ = try await client.loginURL(forSite: "https://wordpress-1315525-4803651.cloudwaysapps.com")
        }, throws: { error in
            #expect(error is LoginError)
            #expect("The certificate for this server is invalid. You might be connecting to a server that is pretending to be “wordpress-1315525-4803651.cloudwaysapps.com” which could put your confidential information at risk." == (error as? LoginError)?.errorDescription)
            return true
        })
    }

    /// This test is unavailable in Linux until https://github.com/swiftlang/swift-corelibs-foundation/pull/4937 lands
    @Test("Login Spec Example 17: Invalid SSL Certificate with explicit exception", .enabled(if: !isLinux()))
    func testInvalidHttpsWithExceptionWorks() async throws {
        let session = URLSession(
            configuration: .default,
            delegate: HTTPSSessionDelegate(
                allowedDomains: ["wordpress-1315525-4803651.cloudwaysapps.com"]
            ),
            delegateQueue: nil
        )
        let client = WordPressLoginClient(urlSession: session)
        _ = try await client.loginURL(forSite: "https://wordpress-1315525-4803651.cloudwaysapps.com")
    }

    final class HTTPSSessionDelegate: NSObject, URLSessionDelegate {

        let allowedDomains: [String]

        init(allowedDomains: [String] = []) {
            self.allowedDomains = allowedDomains
        }

        #if !os(Linux)
        // There's no ability to support self-signed (or otherwise invalid) SSL certificates in Linux until
        // https://github.com/swiftlang/swift-corelibs-foundation/pull/4937 lands.
        func urlSession(
            _ session: URLSession,
            didReceive challenge: URLAuthenticationChallenge,
            completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
        ) {
            guard allowedDomains.contains(challenge.protectionSpace.host),
                  let trust = challenge.protectionSpace.serverTrust else {
                completionHandler(.useCredential, nil)
                return
            }

            completionHandler(.useCredential, URLCredential(trust: trust))
        }
        #endif
    }
}

@Suite("Serialized Login Tests")
struct SerializedLoginTests {
    let client = WordPressLoginClient(urlSession: .shared)
    @Test("Login Spec Example 11: WordPress in a subdirectory with a link tag")
    func testWordPressSubdirectoryWithLinkTag() async throws {
        _ = try await URLSession.shared.data(from: URL(string: "https://subdirectory.wpmt.co/index.php?redirect=true")!)
        let parsedUrl = try await client.loginURL(forSite: "https://subdirectory.wpmt.co/index.php?redirect=true")
        #expect("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 12: WordPress in a subdirectory with a redirect")
    func testWordPressSubdirectory() async throws {
        let parsedUrl = try await client.loginURL(forSite: "https://subdirectory.wpmt.co/index.php?redirect=true")
        #expect("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 15: Rate Limited")
    func testWordPressHeavyRateLimiting() async throws {
        let parsedUrl = try await client.loginURL(forSite: "https://aggressive-rate-limiting.wpmt.co")
        #expect("https://aggressive-rate-limiting.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 15: Rate Limited that never succeeds")
    func testWordPressHeavyRateLimitingThatNeverSucceeds() async throws {

        let stubs = HTTPStubs(stubs: [
            HTTPStubs.stub(host: "aggressive-rate-limiting.wpmt.co", with: .retryAfter(1)),
        ])

        let client = WordPressLoginClient(requestExecutor: stubs)

        await #expect(performing: {
            _ = try await client.loginURL(forSite: "https://aggressive-rate-limiting.wpmt.co")
        }, throws: { error in
            #expect(error is LoginError)
            #expect("The server is rate limiting requests in a way that will never succeed. Please check your site's rate limit configuration." == (error as? LoginError)?.errorDescription)
            return true
        })
    }
}

// swiftlint:enable line_length

