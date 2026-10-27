import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

private let cloudflareTestZone = ProcessInfo.processInfo.environment["WPRS_CLOUDFLARE_TEST_ZONE"]

/// Sends real requests through `URLSession` to a Cloudflare zone set up to produce each error.
/// See `test-data/cloudflare-responses/README.md`.
///
/// `WPRS_CLOUDFLARE_TEST_ZONE=wpmtcf.co swift test --filter CloudflareLiveTests`
@Suite(.enabled(if: cloudflareTestZone != nil, "Needs WPRS_CLOUDFLARE_TEST_ZONE"))
struct CloudflareLiveTests {
    private func api(host: String, apiRoot: String = "/wp-json") throws -> WordPressAPI {
        let base = "https://\(host).\(cloudflareTestZone!)"
        return try WordPressAPI(
            urlSession: URLSession(configuration: .ephemeral),
            siteInfo: .selfHosted(
                siteUrl: ParsedUrl.parse(input: base),
                apiRoot: ParsedUrl.parse(input: base + apiRoot)
            ),
            authenticationProvider: .none()
        )
    }

    struct CloudflareErrorDetails {
        let code: UInt32?
        let kind: CloudflareErrorKind
        let rayId: String?
    }

    private func cloudflareError(_ body: () async throws -> Void) async throws -> CloudflareErrorDetails {
        do {
            try await body()
        } catch let WpApiError.RequestExecutionFailed(_, _, reason, _, _) {
            guard case let .cloudflareError(_, code, kind, rayId, _) = reason else {
                Issue.record("Expected a Cloudflare error, got \(reason)")
                throw reason
            }
            return CloudflareErrorDetails(code: code, kind: kind, rayId: rayId)
        }
        Issue.record("The request was expected to fail")
        throw CancellationError()
    }

    private func expectOriginFailure(host: String, code: UInt32, kind: CloudflareErrorKind) async throws {
        let api = try api(host: host)
        let error = try await cloudflareError { _ = try await api.users.retrieveMeWithEditContext() }
        #expect(error.code == code)
        #expect(error.kind == kind)
        #expect(error.rayId != nil)
    }

    private func expectSecurityRule(path: String, kind: CloudflareErrorKind) async throws {
        let api = try api(host: "cf-rules", apiRoot: path)
        let error = try await cloudflareError { _ = try await api.users.retrieveMeWithEditContext() }
        #expect(error.code == nil)
        #expect(error.kind == kind)
    }

    @Test func originDown() async throws {
        try await expectOriginFailure(host: "cf-521", code: 521, kind: .originDown)
    }

    @Test func originSslHandshakeFailure() async throws {
        try await expectOriginFailure(host: "cf-525", code: 525, kind: .originSsl)
    }

    @Test func originDnsError() async throws {
        try await expectOriginFailure(host: "cf-1016", code: 1016, kind: .originUnreachable)
    }

    @Test func wafBlockPage() async throws {
        try await expectSecurityRule(path: "/cf-test/block", kind: .accessDenied)
    }

    @Test func managedChallenge() async throws {
        try await expectSecurityRule(path: "/cf-test/challenge", kind: .challenge)
    }

    /// What the login screen shows. Discovery sends `Accept: */*`, so Cloudflare answers with its
    /// branded HTML page rather than JSON.
    private func expectLoginMessage(host: String, _ expected: String) async {
        let client = WordPressLoginClient(urlSession: URLSession(configuration: .ephemeral))
        do {
            _ = try await client.details(ofSite: "https://\(host).\(cloudflareTestZone!)")
            Issue.record("Login was expected to fail")
        } catch {
            #expect(error.localizedDescription == expected)
        }
    }

    @Test func loginBehindChallenge() async {
        await expectLoginMessage(host: "cf-challenge-all", "Your site's Cloudflare bot protection is blocking the app.")
    }

    @Test func loginBehindWafBlock() async {
        await expectLoginMessage(host: "cf-block-all", "A Cloudflare security rule on your site blocked this request.")
    }

    @Test func loginWithOriginDnsError() async {
        await expectLoginMessage(host: "cf-1016", "Cloudflare can't reach your site's server.")
    }

    @Test func loginWithOriginDown() async {
        await expectLoginMessage(host: "cf-521", "Cloudflare reports that your site's server is down.")
    }

    /// The CMM-2500 case: a 150 MB upload to a Free-plan zone, whose limit is 100 MB.
    ///
    /// Cloudflare usually answers 413, but sometimes 502 with no code, after the client has sent the
    /// whole body. Either way it's Cloudflare's own page.
    @Test
    func uploadTooLarge() async throws {
        let file = FileManager.default.temporaryDirectory.appending(path: "wprs-cloudflare-150mb.mp4")
        FileManager.default.createFile(atPath: file.path(), contents: nil)
        let handle = try FileHandle(forWritingTo: file)
        try handle.truncate(atOffset: 150 * 1024 * 1024)
        try handle.close()
        defer { try? FileManager.default.removeItem(at: file) }

        let api = try api(host: "testing")
        let error = try await cloudflareError {
            _ = try await api.media.create(params: MediaCreateParams(filePath: file.path()))
        }
        #expect(error.code == nil)
        #expect([.requestTooLarge, .originError].contains(error.kind))
        #expect(error.rayId != nil)
    }
}
