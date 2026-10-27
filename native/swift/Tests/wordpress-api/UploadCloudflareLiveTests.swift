import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

// The CMM-2500 case for real: an upload over the Cloudflare Free plan's 100 MB limit, sent to a zone
// set up for testing Cloudflare's errors. Before `UploadTaskDelegate`, `URLSession` received the
// `413` but timed out about 60 s later over HTTP/3.
//
// Opt-in, because it needs the zone and sends part of a 150 MB file:
//
//     WPRS_CLOUDFLARE_TEST_ZONE=wpmtcf.co BUILDKITE=1 swift test --filter UploadCloudflareLiveTests
#if os(macOS) || os(iOS)
@Suite(
    "Upload to a real Cloudflare zone",
    .enabled(if: ProcessInfo.processInfo.environment["WPRS_CLOUDFLARE_TEST_ZONE"] != nil),
    .timeLimit(.minutes(3))
)
struct UploadCloudflareLiveTests {
    @Test("An upload over Cloudflare's size limit is rejected with a 413, quickly")
    func uploadOverLimitIsRejectedQuickly() async throws {
        let zone = try #require(ProcessInfo.processInfo.environment["WPRS_CLOUDFLARE_TEST_ZONE"])
        let file = FileManager.default.temporaryDirectory.appendingPathComponent("wprs-cloudflare-\(UUID()).mp4")
        try Data().write(to: file)
        try FileHandle(forWritingTo: file).truncate(atOffset: 150 * 1024 * 1024)
        defer { try? FileManager.default.removeItem(at: file) }

        let configuration = URLSessionConfiguration.ephemeral
        // The app's default; the bug was an upload that only failed once this elapsed.
        configuration.timeoutIntervalForRequest = 60
        // A proxy auto-config (PAC) setting makes `URLSession` refuse QUIC, which hides the bug.
        configuration.connectionProxyDictionary = [:]
        let recorder = RecordingDelegate()
        let session = URLSession(configuration: configuration)
        // Cloudflare advertises HTTP/3 with `Alt-Svc`; a first request lets `URLSession` pick it up.
        _ = try await session.data(from: URL(string: "https://testing.\(zone)/wp-json/")!)
        let site = try ParsedUrl.parse(input: "https://testing.\(zone)")
        let api = try WordPressAPI(
            siteInfo: .selfHosted(siteUrl: site, apiRoot: ParsedUrl.parse(input: "https://testing.\(zone)/wp-json")),
            authenticationProvider: .none(),
            executor: WpRequestExecutor(
                urlSession: session,
                notifyingDelegate: recorder
            ),
            middlewarePipeline: .default,
            appNotifier: nil
        )

        let start = ContinuousClock.now
        let result = await Result { try await api.media.create(params: .init(filePath: file.path)) }
        let elapsed = ContinuousClock.now - start
        let events = try await recorder.waitForCompletion()

        let summary = """
            elapsed: \(elapsed)
            protocols: \(events.protocols.map { $0 ?? "nil" })
            responses: \(events.responses)
            bytes sent: \(events.bytesSentAtCompletion.map(String.init) ?? "nil") of \(150 * 1024 * 1024)
            result: \(result)
            """
        print("CLOUDFLARE-LIVE\n\(summary)")

        // How the rejection is classified is up to the Rust layer (#1697 gives Cloudflare's own
        // pages a dedicated reason), so only check that the response, not a timeout, reached it.
        guard case .failure(let error as WpApiError) = result else {
            Issue.record("Expected the upload to be rejected\n\(summary)")
            return
        }
        let statusCode: UInt32? =
            switch error {
            case .UnknownError(let statusCode, _, _, _): statusCode
            case .RequestExecutionFailed(let statusCode, _, _, _, _): statusCode
            default: nil
            }
        // Cloudflare usually answers 413; it sometimes answers 502 instead (see #1697).
        #expect([413, 502].contains(statusCode ?? 0), "\(summary)")
        #expect(elapsed < .seconds(30), "\(summary)")
    }
}

extension UploadCloudflareLiveTests {
    @Test("Over HTTP/3, upload(...) returns Cloudflare's early rejection quickly (CMM-2500)")
    func directUploadOverHTTP3() async throws {
        let zone = try #require(ProcessInfo.processInfo.environment["WPRS_CLOUDFLARE_TEST_ZONE"])
        let file = FileManager.default.temporaryDirectory.appendingPathComponent("wprs-cloudflare-\(UUID()).mp4")
        try Data().write(to: file)
        try FileHandle(forWritingTo: file).truncate(atOffset: 150 * 1024 * 1024)

        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = 60
        configuration.connectionProxyDictionary = [:]
        var request = URLRequest(url: URL(string: "https://testing.\(zone)/wp-json/wp/v2/media")!)
        request.httpMethod = "POST"
        request.setValue("video/mp4", forHTTPHeaderField: "Content-Type")
        request.assumesHTTP3Capable = true
        let recorder = RecordingDelegate()

        let start = ContinuousClock.now
        let result = await Result {
            try await upload(
                body: .onDisk(file),
                with: request,
                session: URLSession(configuration: configuration),
                delegate: recorder
            )
        }
        let elapsed = ContinuousClock.now - start
        let events = try await recorder.waitForCompletion()

        let summary = """
            elapsed: \(elapsed)
            protocols: \(events.protocols.map { $0 ?? "nil" })
            responses: \(events.responses)
            bytes sent: \(events.bytesSentAtCompletion.map(String.init) ?? "nil") of \(150 * 1024 * 1024)
            result: \(result.map { ($0.1 as? HTTPURLResponse)?.statusCode ?? -1 })
            """
        print("CLOUDFLARE-LIVE-H3\n\(summary)")

        // Over HTTP/2 the early response always reached the app, so only HTTP/3 proves the fix. This
        // Mac's proxy auto-config and Tailscale both keep `URLSession` off HTTP/3; turn them off.
        #expect(events.protocols.contains("h3"), "URLSession didn't use HTTP/3\n\(summary)")
        let status = try (result.get().1 as? HTTPURLResponse)?.statusCode
        #expect([413, 502].contains(status ?? 0), "\(summary)")
        #expect(elapsed < .seconds(30), "\(summary)")
        #expect(!FileManager.default.fileExists(atPath: file.path))
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
