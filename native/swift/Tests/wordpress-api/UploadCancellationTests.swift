import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

// Cancelling and timing out an upload: `upload(...)` must resume its caller with the right error on
// every path, and always remove its temp file.
#if os(macOS) || os(iOS)
@Suite("Upload cancellation and timeouts", .serialized, .timeLimit(.minutes(1)))
struct UploadCancellationTests {
    typealias Support = UploadTestSupport

    @Test("Cancelling an in-flight upload throws URLError.cancelled and removes the temp file")
    func cancellingInFlightUpload() async throws {
        let server = try await LoopbackHTTPServer.neverResponding()
        defer { server.stop() }
        let file = try Support.makeTempFile()
        let recorder = RecordingDelegate()

        let uploadTask = Task {
            try await upload(
                body: .onDisk(file),
                with: Support.request(to: server.url),
                session: Support.session(),
                delegate: recorder
            )
        }
        try await server.waitForRequests()
        uploadTask.cancel()

        await #expect(
            performing: { try await uploadTask.value },
            throws: { ($0 as? URLError)?.code == .cancelled }
        )
        #expect(!FileManager.default.fileExists(atPath: file.path))
        let events = try await recorder.waitForCompletion()
        #expect(events.completionErrorCodes == [.cancelled])
    }

    @Test("Cancelling while waiting out an early response's quiet period throws URLError.cancelled")
    func cancellingDuringQuietPeriod() async throws {
        let server = try await LoopbackHTTPServer.start {
            $0.respond(status: 413, body: LoopbackHTTPServer.rejectionBody, includeContentLength: false, closing: false)
        }
        defer { server.stop() }
        let recorder = RecordingDelegate()

        let uploadTask = Task {
            try await upload(
                body: .onDisk(try Support.makeTempFile()),
                with: Support.request(to: server.url),
                session: Support.session(),
                delegate: recorder
            )
        }
        // The response has arrived, but the quiet period hasn't elapsed.
        while recorder.snapshot.receivedData.isEmpty {
            try await Task.sleep(for: .milliseconds(10))
        }
        uploadTask.cancel()

        // The caller cancelled, so the captured response is not returned.
        await #expect(
            performing: { try await uploadTask.value },
            throws: { ($0 as? URLError)?.code == .cancelled }
        )
        let events = try await recorder.waitForCompletion()
        #expect(events.completionErrorCodes == [.cancelled])
    }

    @Test("An upload started from an already-cancelled task is cancelled, not sent")
    func uploadFromCancelledTask() async throws {
        let server = try await LoopbackHTTPServer.neverResponding()
        defer { server.stop() }
        let file = try Support.makeTempFile()

        let start = ContinuousClock.now
        let uploadTask = Task {
            withUnsafeCurrentTask { $0?.cancel() }
            return try await upload(
                body: .onDisk(file),
                with: Support.request(to: server.url),
                session: Support.session(),
                delegate: nil
            )
        }

        await #expect(
            performing: { try await uploadTask.value },
            throws: { ($0 as? URLError)?.code == .cancelled }
        )
        // Not the 10 s request timeout.
        #expect(ContinuousClock.now - start < .seconds(5))
        #expect(!FileManager.default.fileExists(atPath: file.path))
    }

    @Test("A timed-out upload throws URLError.timedOut and removes the temp file")
    func timedOutUpload() async throws {
        let server = try await LoopbackHTTPServer.neverResponding()
        defer { server.stop() }
        let file = try Support.makeTempFile()

        await #expect(
            performing: {
                try await upload(
                    body: .onDisk(file),
                    with: Support.request(to: server.url),
                    session: Support.session(requestTimeout: 0.5),
                    delegate: nil
                )
            },
            throws: { ($0 as? URLError)?.code == .timedOut }
        )
        #expect(!FileManager.default.fileExists(atPath: file.path))
    }

    @Test("A server that reads the body but never answers still times out")
    func serverThatReadsButNeverAnswersTimesOut() async throws {
        let server = try await LoopbackHTTPServer.start { exchange in exchange.readBody { _ in } }
        defer { server.stop() }

        await #expect(
            performing: {
                try await upload(
                    body: .onDisk(try Support.makeTempFile()),
                    with: Support.request(to: server.url),
                    session: Support.session(requestTimeout: 2),
                    delegate: nil
                )
            },
            throws: { ($0 as? URLError)?.code == .timedOut }
        )
        #expect(server.bodyBytesRead > 0)
    }

    // MARK: - Through `WordPressAPI`

    @Test("Cancelling a media upload's Progress surfaces as .cancellationError")
    func cancellingMediaUploadProgress() async throws {
        let server = try await LoopbackHTTPServer.neverResponding()
        defer { server.stop() }
        let file = try Support.makeTempFile()
        defer { try? FileManager.default.removeItem(at: file) }
        let api = try UploadTestSupport.api(for: server)
        let progress = Progress()

        let uploadTask = Task { try await api.uploadMedia(params: .init(filePath: file.path), fulfilling: progress) }
        try await server.waitForRequests()
        progress.cancel()

        await expectReason(.cancellationError) { _ = try await uploadTask.value }
    }

    @Test("Cancelling a media upload's RequestContext surfaces as .cancellationError")
    func cancellingMediaUploadContext() async throws {
        let server = try await LoopbackHTTPServer.neverResponding()
        defer { server.stop() }
        let file = try Support.makeTempFile()
        defer { try? FileManager.default.removeItem(at: file) }
        let executor = WpRequestExecutor(urlSession: Support.session())
        let api = try UploadTestSupport.api(for: server, executor: executor)
        let context = RequestContext()

        let uploadTask = Task {
            try await api.media.createCancellation(params: .init(filePath: file.path), context: context)
        }
        try await server.waitForRequests()
        executor.cancel(context: context)

        await expectReason(.cancellationError) { _ = try await uploadTask.value }
    }

    @Test("A media upload the server never answers surfaces as .httpTimeoutError")
    func unansweredMediaUploadTimesOut() async throws {
        let server = try await LoopbackHTTPServer.neverResponding()
        defer { server.stop() }
        let file = try Support.makeTempFile()
        defer { try? FileManager.default.removeItem(at: file) }
        let api = try UploadTestSupport.api(for: server, requestTimeout: 0.5)

        await expectReason(.httpTimeoutError) { _ = try await api.media.create(params: .init(filePath: file.path)) }
    }

    @Test("A media upload to a server that isn't listening surfaces as .connectionError")
    func mediaUploadToClosedPortIsConnectionError() async throws {
        let server = try await LoopbackHTTPServer.neverResponding()
        server.stop()
        let file = try Support.makeTempFile(size: 1024)
        defer { try? FileManager.default.removeItem(at: file) }
        let api = try UploadTestSupport.api(for: server)

        await expectReason(.connectionError(reason: "")) {
            _ = try await api.media.create(params: .init(filePath: file.path))
        }
    }

    @Test("Upload progress still reaches the Progress passed to uploadMedia")
    func uploadMediaProgressIsReported() async throws {
        let server = try await LoopbackHTTPServer.accepting()
        defer { server.stop() }
        let file = try Support.makeTempFile()
        defer { try? FileManager.default.removeItem(at: file) }
        let progress = Progress()

        // The stub's body isn't a media object, so the call fails to parse; only progress matters.
        _ = try? await UploadTestSupport.api(for: server)
            .uploadMedia(
                params: .init(filePath: file.path),
                fulfilling: progress
            )

        #expect(progress.fractionCompleted > 0.99)
    }
}

/// Expects `body` to throw an error carrying `expected`, comparing only the case.
func expectReason(
    _ expected: RequestExecutionErrorReason,
    sourceLocation: SourceLocation = #_sourceLocation,
    _ body: () async throws -> Void
) async {
    do {
        try await body()
        Issue.record("Expected \(expected), but the call succeeded", sourceLocation: sourceLocation)
    } catch {
        let reason = (error as? CarriesRequestExecutionErrorReason)?.executionErrorReason
        guard let reason, reason.caseName == expected.caseName else {
            Issue.record(
                "Expected \(expected), got \(String(describing: reason)) (error: \(error))",
                sourceLocation: sourceLocation
            )
            return
        }
    }
}

extension RequestExecutionErrorReason {
    /// The case, without its associated values.
    var caseName: String {
        String(String(describing: self).prefix { $0 != "(" })
    }
}

extension UploadTestSupport {
    static func api(
        for server: LoopbackHTTPServer,
        requestTimeout: TimeInterval = requestTimeout,
        executor: WpRequestExecutor? = nil,
        notifyingDelegate: URLSessionTaskDelegate? = nil
    ) throws -> WordPressAPI {
        try WordPressAPI(
            siteInfo: .selfHosted(
                siteUrl: ParsedUrl.parse(input: server.url.absoluteString),
                apiRoot: ParsedUrl.parse(input: server.url.appendingPathComponent("wp-json").absoluteString)
            ),
            authenticationProvider: .none(),
            executor: executor
                ?? WpRequestExecutor(
                    urlSession: session(requestTimeout: requestTimeout),
                    notifyingDelegate: notifyingDelegate
                ),
            middlewarePipeline: .default,
            appNotifier: nil
        )
    }
}
#endif
