import Foundation
import Testing

@testable import WordPressAPI

#if os(Linux)
import FoundationNetworking
#endif

// swiftlint:disable line_length
@Suite("Login")
class LoginTests {

    // swiftlint:disable:next force_try
    let appId = { try! WpUuid.parse(input: "caa8b54a-eb5e-4134-8ae2-a3946a428ec7") }()

    let client = WordPressLoginClient(urlSession: .shared)

    @Test("Login Spec Example 1: Valid URL")
    func testValidURL() async throws {
        let parsedUrl = try await client.loginURL(forSite: "https://vanilla.wpmt.co")
        #expect("https://vanilla.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 2: Local Development Environment")
    func testLocalDevelopmentEnvironment() async throws {
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
        let parsedUrl = try await client.loginURL(forSite: "http://optional-https.wpmt.co")
        #expect("https://optional-https.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 5: HTTP-only site")
    func testHttpOnlySite() async {
        await #expect(performing: {
            // TODO
            try await Task.sleep(nanoseconds: 100)
        }, throws: { error in
            #expect(error is LoginError)
            return true
        })
    }

    @Test("Login Spec Example 6: HTTP-Only Site with Application Password Override")
    func testHttpOnlySiteWithApplicationPasswordsEnabled() async throws {
        let parsedUrl = try await client.loginURL(forSite: "http://http-only.wpmt.co")
        #expect("http://http-only.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
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
            #expect("Unable to login to https://wordfence.wpmt.co – the WordFence plugin might have disabled Application Passwords" == (error as? LoginError)?.errorDescription)
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

    @Test("Login Spec Example 11: WordPress in a subdirectory with a link tag")
    func testWordPressSubdirectoryWithLinkTag() async throws {
        let parsedUrl = try await client.loginURL(forSite: "https://subdirectory.wpmt.co/index.php?link_tag=true")
        #expect("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 12: WordPress in a subdirectory with a redirect")
    func testWordPressSubdirectory() async throws {
        let parsedUrl = try await client.loginURL(forSite: "https://subdirectory.wpmt.co/index.php?redirect=true")
        #expect("https://subdirectory.wpmt.co/wordpress/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 13: Site uses HTTP basic")
    func testWordPressHttpBasic() async throws {
        let parsedUrl = try await client.loginURL(forSite: "https://http-basic-auth.wpmt.co")
        #expect("https://http-basic-auth.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 14: Custom REST API Prefix")
    func testWordPressCustomRestApiPrefix() async throws {
        let parsedUrl = try await client.loginURL(forSite: "https://custom-rest-prefix.wpmt.co")
        #expect("https://custom-rest-prefix.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 15: Rate Limited")
    func testWordPressHeavyRateLimiting() async throws {
        let parsedUrl = try await client.loginURL(forSite: "https://rate-limited.wpmt.co")
        #expect("https://rate-limited.wpmt.co/wp-admin/authorize-application.php" == parsedUrl.url())
    }

    @Test("Login Spec Example 16: Non-existent website")
    func testInvalidUrl() async {
        await #expect(performing: {
            _ = try await client.loginURL(forSite: "https://valid-looking-url-but-not-actually.foo")
        }, throws: { error in
            #expect(error is LoginError)
            #expect("A server with the specified hostname could not be found." == (error as? LoginError)?.errorDescription)
            return true
        })
    }

    @Test("Login Spec Example 17: Invalid SSL Certificate")
    func testInvalidHTTPsFails() async throws {
        let session = URLSession(configuration: .default, delegate: SessionDelegate(), delegateQueue: nil)
        let client = WordPressLoginClient(urlSession: session)
        await #expect(performing: {
            _ = try await client.loginURL(forSite: "https://wordpress-1315525-4803651.cloudwaysapps.com")
        }, throws: { error in
            #expect(error is LoginError)
            return true // TODO: It'd be nice to get more details about this error
        })
    }

    /// This test is unavailable in Linux until https://github.com/swiftlang/swift-corelibs-foundation/pull/4937 lands
    @Test("Login Spec Example 17: Invalid SSL Certificate with explicit exception", .enabled(if: !isLinux()))
    func testInvalidHttpsWithExceptionWorks() async throws {
        let session = URLSession(
            configuration: .default,
            delegate: SessionDelegate(allowedDomains: ["wordpress-1315525-4803651.cloudwaysapps.com"]),
            delegateQueue: nil
        )
        let client = WordPressLoginClient(urlSession: session)
        _ = try await client.loginURL(forSite: "https://wordpress-1315525-4803651.cloudwaysapps.com")
    }

    final class SessionDelegate: NSObject, URLSessionDelegate {

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
// swiftlint:enable line_length
