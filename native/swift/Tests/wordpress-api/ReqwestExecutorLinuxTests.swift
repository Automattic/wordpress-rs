#if os(Linux)
import Foundation
import FoundationNetworking
import Testing

@testable import WordPressAPI

// swiftlint:disable line_length

/// Linux counterpart to `LoginTests.testInvalidHTTPsFails`, which is disabled on
/// Linux because the URLSession executor degrades TLS failures to `.genericError`
/// (Automattic/wordpress-rs#1509). `ReqwestRequestExecutor` classifies them from
/// `rustls`, so on Linux it produces the `.invalidSslError` the Apple suite asserts.
@Suite("Reqwest executor (Linux)")
struct ReqwestExecutorLinuxTests {

    @Test("Invalid SSL certificate is classified, not dropped to GenericError")
    func testInvalidSslIsClassified() async throws {
        let client = WordPressLoginClient(executor: ReqwestRequestExecutor())

        await #expect(
            performing: {
                _ = try await client.details(ofSite: "https://wordpress-1315525-4803651.cloudwaysapps.com")
            },
            throws: { error in
                let reason = try #require(Self.requestExecutionErrorReason(from: error))

                guard case .invalidSslError(let underlyingReason) = reason else {
                    Issue.record("The transport error must be `invalidSslError`, got \(reason)")
                    return false
                }

                guard case .certificateNotValidForName = underlyingReason else {
                    Issue.record("The underlying error must be `certificateNotValidForName`, got \(underlyingReason)")
                    return false
                }

                return true
            }
        )
    }

    /// Pulls the transport reason out of the auto-discovery failure, whichever
    /// stage the TLS error surfaced in (homepage fetch or API-root fetch).
    private static func requestExecutionErrorReason(from error: any Error) -> RequestExecutionErrorReason? {
        guard let failure = error as? AutoDiscoveryAttemptFailure else { return nil }

        if case .FindApiRoot(_, let findFailure) = failure,
            case .fetchHomepage(let transportError) = findFailure,
            case .RequestExecutionFailed(_, _, let reason, _, _) = transportError {
            return reason
        }

        if case .FetchAndParseApiRoot(parsedSiteUrl: _, apiRootUrl: _, let parseFailure) = failure,
            case .fetchApiRoot(let transportError) = parseFailure,
            case .RequestExecutionFailed(_, _, let reason, _, _) = transportError {
            return reason
        }

        return nil
    }
}

// swiftlint:enable line_length
#endif
