import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

// A rejected media upload, end to end through `WordPressAPI.media.create`: the response `upload(...)`
// returns must reach the Rust layer and be classified exactly as it would be had the server read the
// whole body first.
#if os(macOS) || os(iOS)
@Suite("Upload error classification", .serialized, .timeLimit(.minutes(1)))
struct UploadErrorClassificationTests {
    typealias Support = UploadTestSupport

    struct Rejection: CustomTestStringConvertible, Sendable {
        let status: Int
        let headers: [String: String]
        let body: String
        let expected: ExpectedUploadError
        let message: String

        var testDescription: String { "\(status) \(headers["Content-Type"] ?? "")" }

        static let html = ["Content-Type": "text/html"]
        static let json = ["Content-Type": "application/json"]
    }

    static let rejections: [Rejection] = [
        Rejection(
            status: 413,
            headers: Rejection.html,
            body: LoopbackHTTPServer.rejectionBody,
            expected: .unknownError,
            message: "The file is too large for this site's server to accept."
        ),
        Rejection(
            status: 413,
            headers: Rejection.json,
            body: #"{"code":"rest_upload_file_too_big","message":"This file is too big."}"#,
            expected: .wpError(code: "rest_upload_file_too_big"),
            message: "This file is too big."
        ),
        Rejection(
            status: 401,
            headers: Rejection.json,
            body: #"{"code":"rest_cannot_create","message":"Sorry, you are not allowed to upload files."}"#,
            expected: .wpError(code: "rest_cannot_create"),
            message: "Sorry, you are not allowed to upload files."
        ),
        Rejection(
            status: 401,
            headers: Rejection.html.merging(["WWW-Authenticate": #"Basic realm="site""#]) { $1 },
            body: "<html>401</html>",
            expected: .reason(.httpAuthenticationRequiredError(hostname: "", method: nil)),
            message: ""
        ),
        Rejection(
            status: 403,
            headers: Rejection.html,
            body: "<html>403 Forbidden</html>",
            expected: .reason(.httpForbiddenError(hostname: "")),
            message: ""
        ),
        Rejection(
            status: 500,
            headers: Rejection.html,
            body: "<html>500</html>",
            expected: .unknownError,
            message: "Something went wrong."
        ),
        Rejection(
            status: 502,
            headers: Rejection.html,
            body: "<html>502 Bad Gateway</html>",
            expected: .unknownError,
            message: "Something went wrong."
        )
    ]

    @Test("A rejection sent before the body is read is classified", arguments: rejections)
    func earlyRejectionIsClassified(rejection: Rejection) async throws {
        let server = try await LoopbackHTTPServer.start {
            $0.respond(status: rejection.status, headers: rejection.headers, body: rejection.body, closing: false)
        }
        defer { server.stop() }

        try await expectClassification(rejection, uploadingTo: server)
        #expect(server.bodyBytesRead < Support.largeBodySize / 2)
    }

    @Test("A rejection sent after the body is read is classified the same way", arguments: rejections)
    func lateRejectionIsClassified(rejection: Rejection) async throws {
        let server = try await LoopbackHTTPServer.start { exchange in
            exchange.readBody { _ in
                exchange.respond(status: rejection.status, headers: rejection.headers, body: rejection.body)
            }
        }
        defer { server.stop() }

        try await expectClassification(rejection, uploadingTo: server)
        #expect(server.bodyBytesRead >= Support.largeBodySize)
    }

    @Test("A small media upload rejected without being read is classified")
    func smallEarlyRejectionIsClassified() async throws {
        // Small enough to be sent in memory rather than from a temp file.
        let server = try await LoopbackHTTPServer.rejectingEarly()
        defer { server.stop() }

        try await expectClassification(Self.rejections[0], uploadingTo: server, fileSize: 64 * 1024)
    }

    @Test("A successful response reaches the Rust layer")
    func successResponseReachesRust() async throws {
        let server = try await LoopbackHTTPServer.accepting(body: #"{"not":"a media object"}"#)
        defer { server.stop() }
        let file = try Support.makeTempFile()
        defer { try? FileManager.default.removeItem(at: file) }

        // The body isn't a media object, so it fails to parse — proving it got as far as parsing.
        do {
            _ = try await Support.api(for: server).media.create(params: .init(filePath: file.path))
            Issue.record("Expected a parsing error")
        } catch let error as WpApiError {
            guard case .ResponseParsingError(_, let response, _, _) = error else {
                Issue.record("Expected .ResponseParsingError, got \(error)")
                return
            }
            #expect(response == #"{"not":"a media object"}"#)
        }
    }

    @Test("An upload's redirect is recorded by the executor")
    func uploadRedirectIsRecorded() async throws {
        // `UploadTaskDelegate` forwards redirects to `RequestExecutorDelegate` through Objective-C
        // message forwarding. Redirecting to a closed port makes the upload fail with an error that
        // carries the redirects the executor recorded.
        let closed = try await LoopbackHTTPServer.neverResponding()
        closed.stop()
        let destination = closed.url.appendingPathComponent("elsewhere").absoluteString
        let server = try await LoopbackHTTPServer.start { exchange in
            exchange.readBody { _ in exchange.respond(status: 307, headers: ["Location": destination]) }
        }
        defer { server.stop() }
        let file = try Support.makeTempFile(size: 256 * 1024)
        defer { try? FileManager.default.removeItem(at: file) }

        do {
            _ = try await Support.api(for: server).media.create(params: .init(filePath: file.path))
            Issue.record("Expected a connection error")
        } catch let error as WpApiError {
            guard case .RequestExecutionFailed(_, let redirects, let reason, _, _) = error else {
                Issue.record("Expected .RequestExecutionFailed, got \(error)")
                return
            }
            #expect(reason.caseName == "connectionError")
            #expect(redirects?.map(\.destination) == [destination])
        }
    }

    @Test("Through WpRequestExecutor, the app's delegate sees an early rejection once")
    func executorNotifiesOnceForEarlyRejection() async throws {
        let server = try await LoopbackHTTPServer.rejectingEarly()
        defer { server.stop() }
        let file = try Support.makeTempFile()
        defer { try? FileManager.default.removeItem(at: file) }
        let recorder = RecordingDelegate()

        _ = try? await Support.api(for: server, notifyingDelegate: recorder).media
            .create(
                params: .init(filePath: file.path)
            )

        let events = try await recorder.waitForCompletion()
        #expect(events.createdTasks == 1)
        #expect(events.responses == [413])
        #expect(events.receivedData == Data(LoopbackHTTPServer.rejectionBody.utf8))
        #expect(events.completions.count == 1)
        #expect(events.completions.allSatisfy { $0 == nil })
        #expect(events.metrics == 1)
    }

    @Test("Through WpRequestExecutor, the app's delegate sees a normal upload once")
    func executorNotifiesOnceForNormalUpload() async throws {
        let server = try await LoopbackHTTPServer.accepting()
        defer { server.stop() }
        let file = try Support.makeTempFile()
        defer { try? FileManager.default.removeItem(at: file) }
        let recorder = RecordingDelegate()

        _ = try? await Support.api(for: server, notifyingDelegate: recorder).media
            .create(
                params: .init(filePath: file.path)
            )

        let events = try await recorder.waitForCompletion()
        #expect(events.createdTasks == 1)
        #expect(events.responses == [201])
        #expect(events.completions.count == 1)
        #expect(events.completions.allSatisfy { $0 == nil })
    }

    @Test("Through WpRequestExecutor, the app's delegate sees a timeout once")
    func executorNotifiesOnceForTimeout() async throws {
        let server = try await LoopbackHTTPServer.neverResponding()
        defer { server.stop() }
        let file = try Support.makeTempFile()
        defer { try? FileManager.default.removeItem(at: file) }
        let recorder = RecordingDelegate()

        _ = try? await Support.api(for: server, requestTimeout: 0.5, notifyingDelegate: recorder).media
            .create(
                params: .init(filePath: file.path)
            )

        let events = try await recorder.waitForCompletion()
        #expect(events.completionErrorCodes == [.timedOut])
    }

    // MARK: - Helpers

    private func expectClassification(
        _ rejection: Rejection,
        uploadingTo server: LoopbackHTTPServer,
        fileSize: Int = UploadTestSupport.largeBodySize,
        sourceLocation: SourceLocation = #_sourceLocation
    ) async throws {
        let file = try Support.makeTempFile(size: fileSize)
        defer { try? FileManager.default.removeItem(at: file) }

        do {
            _ = try await Support.api(for: server).media.create(params: .init(filePath: file.path))
            Issue.record("Expected the upload to be rejected", sourceLocation: sourceLocation)
            return
        } catch let error as WpApiError {
            #expect(error.status == UInt32(rejection.status), sourceLocation: sourceLocation)

            switch (rejection.expected, error) {
            case (.unknownError, .UnknownError):
                break
            case (.wpError(let expectedCode), .WpError(_, _, _, let response, _, _)):
                #expect(response.contains(#""code":"\#(expectedCode)""#), sourceLocation: sourceLocation)
            case (.reason(let expected), .RequestExecutionFailed(_, _, let reason, _, _)):
                #expect(reason.caseName == expected.caseName, sourceLocation: sourceLocation)
            default:
                Issue.record("Expected \(rejection.expected), got \(error)", sourceLocation: sourceLocation)
            }

            if !rejection.message.isEmpty {
                #expect(error.localizedDescription == rejection.message, sourceLocation: sourceLocation)
            }
        }
    }
}

/// How a rejected upload should be classified.
enum ExpectedUploadError: Sendable {
    case unknownError
    case wpError(code: String)
    case reason(RequestExecutionErrorReason)
}

extension WpApiError {
    /// The HTTP status code the error carries, if any.
    fileprivate var status: UInt32? {
        switch self {
        case .UnknownError(let statusCode, _, _, _), .InvalidHttpStatusCode(let statusCode, _, _):
            statusCode
        case .WpError(_, _, let statusCode, _, _, _):
            statusCode
        case .RequestExecutionFailed(let statusCode, _, _, _, _):
            statusCode
        default:
            nil
        }
    }
}
#endif
