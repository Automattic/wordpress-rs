import Foundation
import Testing

@testable import WordPressAPI

#if os(Linux)
import FoundationNetworking
#endif

@Suite("Login")
class LoginTests {

    // swiftlint:disable:next force_try
    let appId = { try! WpUuid.parse(input: "caa8b54a-eb5e-4134-8ae2-a3946a428ec7") }()

    @Test
    func testInvalidUrl() async {
        let client = WordPressLoginClient(urlSession: .shared)

        await #expect(performing: {
            _ = try await client.loginURL(forSite: "invalid url")
        }, throws: { error in
            return true
        })
    }

    @Test
    func testNotWordPressSite() async throws {
        let client = WordPressLoginClient(urlSession: .shared)

        await #expect(performing: {
            let parsedUrl = try await client.loginURL(forSite: "https://example.com/blog")
        }, throws: { error in
            return true
        })
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
            completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void)
        {
            guard allowedDomains.contains(challenge.protectionSpace.host),
                  let trust = challenge.protectionSpace.serverTrust else {
                completionHandler(.useCredential, nil)
                return
            }

            completionHandler(.useCredential, URLCredential(trust: trust))
        }
        #endif
    }

    @Test
    func testInvalidHTTPsFails() async throws {
        let session = URLSession(configuration: .default, delegate: SessionDelegate(), delegateQueue: nil)
        let client = WordPressLoginClient(urlSession: session)
        await #expect(performing: {
            _ = try await client.loginURL(forSite: "https://wordpress-1315525-4803651.cloudwaysapps.com")
        }, throws: { error in
            true // TODO: It'd be nice to get more details about this error
        })
    }

    /// This test is unavailable in Linux until https://github.com/swiftlang/swift-corelibs-foundation/pull/4937 lands
    @Test("We can set exception domains to allow invalid SSL certs", .enabled(if: !isLinux()))
    func testInvalidHttpsWithExceptionWorks() async throws {
        let session = URLSession(
            configuration: .default,
            delegate: SessionDelegate(allowedDomains: ["wordpress-1315525-4803651.cloudwaysapps.com"]),
            delegateQueue: nil
        )
        let client = WordPressLoginClient(urlSession: session)
        _ = try await client.loginURL(forSite: "https://wordpress-1315525-4803651.cloudwaysapps.com")
    }
}
