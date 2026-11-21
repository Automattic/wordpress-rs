import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

// The CMM-2500 case for real: an upload over the Cloudflare Free plan's 100 MB limit, sent to a zone
// set up for testing Cloudflare's errors. Over HTTP/3, Cloudflare answers with a `413` after a few MB.
// Before `UploadTaskDelegate`, `URLSession` received it but timed out about 60 s later.
//
// Opt-in, because it needs the zone and sends part of a 150 MB file:
//
//     WPRS_CLOUDFLARE_TEST_ZONE=wpmtcf.co BUILDKITE=1 swift test --filter UploadCloudflareLiveTests
//
// Only HTTP/3 reproduces the bug. On a Mac with a proxy auto-config (PAC) setting, turn it off:
// it makes `URLSession` refuse QUIC.
#if os(macOS) || os(iOS)
@Suite(
    "Upload to a real Cloudflare zone",
    .serialized,
    .enabled(if: ProcessInfo.processInfo.environment["WPRS_CLOUDFLARE_TEST_ZONE"] != nil),
    .timeLimit(.minutes(5))
)
struct UploadCloudflareLiveTests {
    /// Cloudflare sometimes resets the request instead of answering (`-1017` over HTTP/3), which
    /// fails fast but proves nothing, so a test retries that up to this many times.
    static let attempts = 3

    /// The app's default. The bug was an upload that only failed once this elapsed.
    static let requestTimeout: TimeInterval = 60

    static let fileSize = 150 * 1024 * 1024

    @Test("Over HTTP/3, upload(...) returns Cloudflare's early rejection quickly (CMM-2500)")
    func directUploadOverHTTP3() async throws {
        let zone = try Self.zone()
        var request = URLRequest(url: URL(string: "https://testing.\(zone)/wp-json/wp/v2/media")!)
        request.httpMethod = "POST"
        request.setValue("video/mp4", forHTTPHeaderField: "Content-Type")
        request.assumesHTTP3Capable = true

        for attempt in 1...Self.attempts {
            let file = try Self.makeFile()
            let recorder = RecordingDelegate()
            let start = ContinuousClock.now
            let result = await Result {
                try await upload(body: .onDisk(file), with: request, session: Self.session(), delegate: recorder)
            }
            let elapsed = ContinuousClock.now - start
            let events = try await recorder.waitForCompletion()
            let summary = Self.summary(attempt, elapsed, events, result.map { ($0.1 as? HTTPURLResponse)?.statusCode })
            print(summary)

            #expect(elapsed < .seconds(30), "\(summary)")
            #expect(!FileManager.default.fileExists(atPath: file.path))
            if case .failure(let error as URLError) = result, error.code == .cannotParseResponse {
                continue
            }

            // Over HTTP/2 the early response always reached the app, so only HTTP/3 proves the fix.
            #expect(events.protocols.contains("h3"), "URLSession didn't use HTTP/3\n\(summary)")
            let status = try (result.get().1 as? HTTPURLResponse)?.statusCode
            // Cloudflare usually answers 413; it sometimes answers 502 instead (see #1697).
            #expect([413, 502].contains(status ?? 0), "\(summary)")
            return
        }
        Issue.record("Cloudflare reset the request without a response on every attempt")
    }

    @Test("Through WordPressAPI, an upload over Cloudflare's size limit is rejected quickly")
    func mediaUploadIsRejectedQuickly() async throws {
        let zone = try Self.zone()
        let session = Self.session()
        // Cloudflare advertises HTTP/3 with `Alt-Svc`; a first request lets `URLSession` pick it up.
        _ = try await session.data(from: URL(string: "https://testing.\(zone)/wp-json/")!)

        for attempt in 1...Self.attempts {
            let file = try Self.makeFile()
            defer { try? FileManager.default.removeItem(at: file) }
            let recorder = RecordingDelegate()
            let api = try WordPressAPI(
                siteInfo: .selfHosted(
                    siteUrl: ParsedUrl.parse(input: "https://testing.\(zone)"),
                    apiRoot: ParsedUrl.parse(input: "https://testing.\(zone)/wp-json")
                ),
                authenticationProvider: .none(),
                executor: WpRequestExecutor(urlSession: session, notifyingDelegate: recorder),
                middlewarePipeline: .default,
                appNotifier: nil
            )

            let start = ContinuousClock.now
            let result = await Result { try await api.media.create(params: .init(filePath: file.path)) }
            let elapsed = ContinuousClock.now - start
            let events = try await recorder.waitForCompletion()
            let summary = Self.summary(attempt, elapsed, events, result)
            print(summary)

            #expect(elapsed < .seconds(30), "\(summary)")
            guard case .failure(let error as WpApiError) = result else {
                Issue.record("Expected the upload to be rejected\n\(summary)")
                return
            }
            if events.completionErrorCodes == [.cannotParseResponse] {
                continue
            }

            // How the rejection is classified is up to the Rust layer (#1697 gives Cloudflare's own
            // pages a dedicated reason), so only check that the response, not a timeout, reached it.
            let statusCode: UInt32? =
                switch error {
                case .UnknownError(let statusCode, _, _, _): statusCode
                case .RequestExecutionFailed(let statusCode, _, _, _, _): statusCode
                default: nil
                }
            #expect([413, 502].contains(statusCode ?? 0), "\(summary)")
            return
        }
        Issue.record("Cloudflare reset the request without a response on every attempt")
    }

    // MARK: - Helpers

    private static func zone() throws -> String {
        try #require(ProcessInfo.processInfo.environment["WPRS_CLOUDFLARE_TEST_ZONE"])
    }

    /// A sparse 150 MB file, so creating it is instant.
    private static func makeFile() throws -> URL {
        try UploadTestSupport.makeTempFile(size: fileSize)
    }

    private static func session() -> URLSession {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = requestTimeout
        // Bypass any proxy for this session. It doesn't bypass a system PAC setting's effect on QUIC.
        configuration.connectionProxyDictionary = [:]
        return URLSession(configuration: configuration)
    }

    private static func summary<T>(
        _ attempt: Int,
        _ elapsed: Duration,
        _ events: RecordingDelegate.Events,
        _ result: Result<T, any Error>
    ) -> String {
        """
        CLOUDFLARE-LIVE attempt \(attempt)
          elapsed: \(elapsed)
          protocols: \(events.protocols.map { $0 ?? "nil" })
          responses: \(events.responses)
          completion: \(events.completionErrorCodes.map { $0.map { "\($0.rawValue)" } ?? "nil" })
          bytes sent: \(events.bytesSentAtCompletion.map(String.init) ?? "nil") of \(fileSize)
          result: \(String(describing: result).prefix(160))
        """
    }
}

extension Result where Failure == any Error {
    fileprivate init(catching body: () async throws -> Success) async {
        do {
            self = .success(try await body())
        } catch {
            self = .failure(error)
        }
    }
}
#endif
