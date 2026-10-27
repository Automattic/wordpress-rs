import Foundation
import Testing
@testable import WordPressAPI
@testable import WordPressAPIInternal

// A server can answer an upload before it has read the whole body — Cloudflare rejects an oversized
// upload with a `413` after about 1–2 MB (CMM-2500). `URLSession` receives that response, but a
// completion-handler task never completes: it waits to send the rest of the body and then times out,
// so the app reported `httpTimeoutError` instead of the `413`.
//
// These tests drive `upload(...)` against `LoopbackHTTPServer`, a real HTTP/1.1 server that can
// answer without reading the body and keep the connection open. That hangs `URLSession` the same way
// Cloudflare does over HTTP/3.
//
// Gated to macOS and iOS, like the other `upload(...)` tests in `MultipartFormTests`.
#if os(macOS) || os(iOS)
@Suite("Upload early responses", .serialized, .timeLimit(.minutes(1)))
struct UploadEarlyResponseTests {
    typealias Support = UploadTestSupport

    // MARK: - Returning the early response

    @Test("An early 413 is returned without waiting for the request timeout")
    func earlyResponseWithContentLengthIsReturned() async throws {
        let server = try await LoopbackHTTPServer.rejectingEarly()
        defer { server.stop() }
        let file = try Support.makeTempFile()

        let start = ContinuousClock.now
        let (data, response) = try await upload(
            body: .onDisk(file),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 413)
        #expect(String(bytes: data, encoding: .utf8) == LoopbackHTTPServer.rejectionBody)
        #expect(ContinuousClock.now - start < .seconds(5))
        #expect(!FileManager.default.fileExists(atPath: file.path))
    }

    @Test("The early-response tests model Cloudflare: most of the body is never read")
    func earlyResponseLeavesBodyUnsent() async throws {
        let server = try await LoopbackHTTPServer.rejectingEarly()
        defer { server.stop() }

        _ = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect(server.bodyBytesRead < Support.largeBodySize / 2)
    }

    @Test("An early response from an in-memory body is returned")
    func earlyResponseFromInMemoryBodyIsReturned() async throws {
        let server = try await LoopbackHTTPServer.rejectingEarly()
        defer { server.stop() }

        let (_, response) = try await upload(
            body: .inMemory(Data(count: Support.largeBodySize)),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 413)
    }

    @Test("A small upload rejected without being read is returned")
    func smallBodyRejectedWithoutReadingIsReturned() async throws {
        // Small enough to fit in the socket buffers, so `URLSession` may consider it fully sent.
        let server = try await LoopbackHTTPServer.rejectingEarly()
        defer { server.stop() }

        let (data, response) = try await upload(
            body: .inMemory(Data(count: 16 * 1024)),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 413)
        #expect(String(bytes: data, encoding: .utf8) == LoopbackHTTPServer.rejectionBody)
    }

    @Test("An early response with an empty body still times out")
    func earlyResponseWithEmptyBodyTimesOut() async throws {
        let server = try await LoopbackHTTPServer.start { $0.respond(status: 413, closing: false) }
        defer { server.stop() }

        // `URLSession` never calls `didReceive response` for it: it reports the whole body as sent and
        // waits, so there's nothing for `UploadTaskDelegate` to return.
        await withKnownIssue("URLSession doesn't deliver an early response with an empty body", isIntermittent: true) {
            let (_, response) = try await upload(
                body: .onDisk(try Support.makeTempFile()),
                with: Support.request(to: server.url),
                session: Support.session(requestTimeout: 1),
                delegate: nil
            )
            #expect((response as? HTTPURLResponse)?.statusCode == 413)
        }
    }

    @Test("An early response with neither a Content-Length nor a Content-Type still times out")
    func earlyResponseWithoutLengthOrTypeTimesOut() async throws {
        let server = try await LoopbackHTTPServer.start {
            $0.send("HTTP/1.1 413 Payload Too Large\r\n\r\n" + LoopbackHTTPServer.rejectionBody)
        }
        defer { server.stop() }

        // `URLSession` holds the response back to sniff its type from more of the body.
        await withKnownIssue("URLSession doesn't deliver an untyped early response of unknown length") {
            let (_, response) = try await upload(
                body: .onDisk(try Support.makeTempFile()),
                with: Support.request(to: server.url),
                session: Support.session(requestTimeout: 1),
                delegate: nil
            )
            #expect((response as? HTTPURLResponse)?.statusCode == 413)
        }
    }

    @Test("An early success response is returned like any other")
    func earlySuccessResponseIsReturned() async throws {
        let server = try await LoopbackHTTPServer.start {
            $0.respond(status: 200, headers: ["Content-Type": "application/json"], body: "{}", closing: false)
        }
        defer { server.stop() }

        let (data, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 200)
        #expect(String(bytes: data, encoding: .utf8) == "{}")
    }

    @Test("An early response keeps its headers")
    func earlyResponseKeepsHeaders() async throws {
        let server = try await LoopbackHTTPServer.start {
            $0.respond(
                status: 413,
                headers: ["CF-Ray": "abc123-YVR"],
                body: LoopbackHTTPServer.rejectionBody,
                closing: false
            )
        }
        defer { server.stop() }

        let (_, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        let httpResponse = try #require(response as? HTTPURLResponse)
        #expect(httpResponse.value(forHTTPHeaderField: "CF-Ray") == "abc123-YVR")
        #expect(httpResponse.value(forHTTPHeaderField: "Content-Type") == "text/html")
    }

    // MARK: - Waiting for the whole early response body

    @Test("An early response without a Content-Length is returned after the quiet period")
    func earlyResponseWithoutContentLengthIsReturned() async throws {
        let server = try await LoopbackHTTPServer.start {
            $0.respond(status: 413, body: LoopbackHTTPServer.rejectionBody, includeContentLength: false, closing: false)
        }
        defer { server.stop() }

        let start = ContinuousClock.now
        let (data, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )
        let elapsed = ContinuousClock.now - start

        #expect((response as? HTTPURLResponse)?.statusCode == 413)
        #expect(String(bytes: data, encoding: .utf8) == LoopbackHTTPServer.rejectionBody)
        #expect(elapsed >= .milliseconds(900))
        #expect(elapsed < .seconds(5))
    }

    // `URLSession` buffers the first 512 bytes of a response body (to sniff its type) before
    // delivering any of it, so the parts below are larger than that, or they'd arrive as one.

    @Test("Each new part of an early response without a Content-Length restarts the quiet period")
    func earlyResponseWithoutContentLengthWaitsForEveryPart() async throws {
        let parts = (0..<3).map { String(repeating: Character("\($0)"), count: 32 * 1024) }
        let server = try await LoopbackHTTPServer.start { exchange in
            exchange.send("HTTP/1.1 413 Payload Too Large\r\nContent-Type: text/html\r\n\r\n" + parts[0])
            // Each gap is shorter than the quiet period; together they're longer.
            exchange.after(.milliseconds(600)) { exchange.send(parts[1]) }
            exchange.after(.milliseconds(1200)) { exchange.send(parts[2]) }
        }
        defer { server.stop() }
        let recorder = RecordingDelegate()

        let (data, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: recorder
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 413)
        #expect(String(bytes: data, encoding: .utf8) == parts.joined())
        // The parts really did arrive separately.
        #expect(recorder.snapshot.dataCallbacks >= 2)
    }

    @Test("An early response with a Content-Length waits for the whole body, however slowly it arrives")
    func earlyResponseWithContentLengthWaitsForWholeBody() async throws {
        let parts = [String(repeating: "a", count: 32 * 1024), String(repeating: "b", count: 32 * 1024)]
        let server = try await LoopbackHTTPServer.start { exchange in
            let head = "HTTP/1.1 413 Payload Too Large\r\nContent-Type: text/html\r\nContent-Length: 65536\r\n\r\n"
            exchange.send(head + parts[0])
            // Longer than the quiet period: the `Content-Length` says more is coming.
            exchange.after(.milliseconds(1500)) { exchange.send(parts[1]) }
        }
        defer { server.stop() }

        let recorder = RecordingDelegate()

        let (data, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: recorder
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 413)
        #expect(String(bytes: data, encoding: .utf8) == parts.joined())
        #expect(recorder.snapshot.dataCallbacks >= 2)
    }

    // MARK: - Preferring the early response over a later error

    @Test("An early response is returned even if the connection then breaks")
    func earlyResponseIsPreferredOverConnectionLoss() async throws {
        let server = try await LoopbackHTTPServer.start { exchange in
            // A chunked body cut off before its last chunk is an error (`-1005`); a short
            // `Content-Length` body isn't — `URLSession` reports that as a success.
            let head = "HTTP/1.1 413 Payload Too Large\r\nContent-Type: text/html\r\nTransfer-Encoding: chunked\r\n\r\n"
            exchange.send(head + "6\r\n<html>\r\n")
            exchange.after(.milliseconds(200)) { exchange.disconnect() }
        }
        defer { server.stop() }
        let file = try Support.makeTempFile()

        let (data, response) = try await upload(
            body: .onDisk(file),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 413)
        #expect(String(bytes: data, encoding: .utf8) == "<html>")
        #expect(!FileManager.default.fileExists(atPath: file.path))
    }

    @Test("An early response is returned even if the request then times out")
    func earlyResponseIsPreferredOverTimeout() async throws {
        let server = try await LoopbackHTTPServer.start { exchange in
            // Promises more of the body than it ever sends.
            let head = "HTTP/1.1 413 Payload Too Large\r\nContent-Type: text/html\r\nContent-Length: 4096\r\n\r\n"
            exchange.send(head + String(repeating: "x", count: 1024))
        }
        defer { server.stop() }

        let (data, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(requestTimeout: 2),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 413)
        #expect(data == Data(String(repeating: "x", count: 1024).utf8))
    }

    // MARK: - Normal responses

    @Test("A response after the full body is returned unchanged, and the server receives every byte")
    func normalResponseIsUnchanged() async throws {
        let server = try await LoopbackHTTPServer.accepting()
        defer { server.stop() }
        let file = try Support.makeTempFile()

        let (data, response) = try await upload(
            body: .onDisk(file),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 201)
        #expect(String(bytes: data, encoding: .utf8) == #"{"accepted":true}"#)
        #expect(server.bodyBytesRead == Support.largeBodySize)
        #expect(!FileManager.default.fileExists(atPath: file.path))
    }

    @Test("A small in-memory upload is returned unchanged")
    func smallInMemoryUploadIsUnchanged() async throws {
        let server = try await LoopbackHTTPServer.accepting()
        defer { server.stop() }

        let (data, response) = try await upload(
            body: .inMemory(Data("hello".utf8)),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 201)
        #expect(String(bytes: data, encoding: .utf8) == #"{"accepted":true}"#)
        #expect(server.bodyBytesRead == 5)
    }

    @Test("A rejection after the full body is read is returned unchanged")
    func lateRejectionIsUnchanged() async throws {
        let server = try await LoopbackHTTPServer.start { exchange in
            exchange.readBody { _ in exchange.respond(status: 413, body: LoopbackHTTPServer.rejectionBody) }
        }
        defer { server.stop() }

        let (data, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 413)
        #expect(String(bytes: data, encoding: .utf8) == LoopbackHTTPServer.rejectionBody)
        #expect(server.bodyBytesRead == Support.largeBodySize)
    }

    @Test("A chunked response after the full body is returned unchanged")
    func chunkedNormalResponseIsUnchanged() async throws {
        let server = try await LoopbackHTTPServer.start { exchange in
            exchange.readBody { _ in
                let head =
                    "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n"
                exchange.send(head + "3\r\n{\"a\r\n")
                exchange.after(.milliseconds(300)) { exchange.send("2\r\n\"}\r\n0\r\n\r\n") }
            }
        }
        defer { server.stop() }

        let (data, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 201)
        #expect(String(bytes: data, encoding: .utf8) == #"{"a"}"#)
    }

    @Test("An empty 204 response is returned unchanged")
    func emptyNormalResponseIsUnchanged() async throws {
        let server = try await LoopbackHTTPServer.start { exchange in
            exchange.readBody { _ in exchange.respond(status: 204) }
        }
        defer { server.stop() }

        let (data, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 204)
        #expect(data.isEmpty)
    }

    @Test("A large response body is returned intact")
    func largeResponseBodyIsIntact() async throws {
        let body = String(repeating: "0123456789abcdef", count: 128 * 1024)
        let server = try await LoopbackHTTPServer.start { exchange in
            exchange.readBody { _ in exchange.respond(status: 200, headers: ["Content-Type": "text/plain"], body: body)
            }
        }
        defer { server.stop() }

        let (data, _) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: server.url),
            session: Support.session(),
            delegate: nil
        )

        #expect(data.count == body.utf8.count)
        #expect(String(bytes: data, encoding: .utf8) == body)
    }

    // MARK: - Concurrency and races

    @Test("Concurrent early responses on one session are each returned to their own caller")
    func concurrentEarlyResponses() async throws {
        let server = try await LoopbackHTTPServer.start { exchange in
            exchange.respond(
                status: 413,
                headers: ["Connection": "close"],
                body: "rejected \(exchange.request.path)",
                closing: false
            )
        }
        defer { server.stop() }
        let session = Support.session()

        let results = try await withThrowingTaskGroup(of: (Int, Int?, String?, Bool).self) { group in
            for index in 0..<8 {
                group.addTask {
                    let file = try Support.makeTempFile()
                    let (data, response) = try await upload(
                        body: .onDisk(file),
                        with: Support.request(to: server.url, path: "/upload/\(index)"),
                        session: session,
                        delegate: nil
                    )
                    return (
                        index,
                        (response as? HTTPURLResponse)?.statusCode,
                        String(bytes: data, encoding: .utf8),
                        FileManager.default.fileExists(atPath: file.path)
                    )
                }
            }
            return try await group.reduce(into: []) { $0.append($1) }
        }

        #expect(results.count == 8)
        for (index, status, body, fileExists) in results {
            #expect(status == 413)
            #expect(body == "rejected /upload/\(index)")
            #expect(!fileExists)
        }
    }

    @Test("Uploads queued behind an early response on a kept-alive connection can fail")
    func queuedUploadsBehindKeptAliveEarlyResponse() async throws {
        let server = try await LoopbackHTTPServer.rejectingEarly(keepAlive: true)
        defer { server.stop() }
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = Support.requestTimeout
        configuration.httpMaximumConnectionsPerHost = 1
        let session = URLSession(configuration: configuration)
        let body = Data(count: Support.largeBodySize)

        // `URLSession` gives the next upload the connection of the one that was cancelled. See
        // `UploadTaskDelegate`: a server that sends `Connection: close` avoids this.
        let issue: Comment = "URLSession reuses a kept-alive connection after an early response"
        await withKnownIssue(issue, isIntermittent: true) {
            try await withThrowingTaskGroup(of: Void.self) { group in
                for _ in 0..<4 {
                    group.addTask {
                        let (_, response) = try await upload(
                            body: .inMemory(body),
                            with: Support.request(to: server.url),
                            session: session,
                            delegate: nil
                        )
                        #expect((response as? HTTPURLResponse)?.statusCode == 413)
                    }
                }
                try await group.waitForAll()
            }
        }
    }

    @Test("Uploads queued behind an early response with Connection: close each get the response")
    func queuedUploadsBehindClosingEarlyResponse() async throws {
        let server = try await LoopbackHTTPServer.rejectingEarly()
        defer { server.stop() }
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = Support.requestTimeout
        configuration.httpMaximumConnectionsPerHost = 1
        let session = URLSession(configuration: configuration)
        let body = Data(count: Support.largeBodySize)

        try await withThrowingTaskGroup(of: Void.self) { group in
            for _ in 0..<4 {
                group.addTask {
                    let (_, response) = try await upload(
                        body: .inMemory(body),
                        with: Support.request(to: server.url),
                        session: session,
                        delegate: nil
                    )
                    #expect((response as? HTTPURLResponse)?.statusCode == 413)
                }
            }
            try await group.waitForAll()
        }
        #expect(server.requests.count == 4)
    }

    @Test("A complete early response racing a dropped connection resumes the caller exactly once")
    func earlyResponseRacingDisconnect() async throws {
        // `CheckedContinuation` traps on a second resume, so a lost race crashes the test run.
        let server = try await LoopbackHTTPServer.start { exchange in
            exchange.respond(status: 413, body: LoopbackHTTPServer.rejectionBody, closing: false) {
                exchange.disconnect()
            }
        }
        defer { server.stop() }
        let session = Support.session()
        let body = Data(count: Support.largeBodySize)

        for _ in 0..<25 {
            do {
                let (_, response) = try await upload(
                    body: .inMemory(body),
                    with: Support.request(to: server.url),
                    session: session,
                    delegate: nil
                )
                #expect((response as? HTTPURLResponse)?.statusCode == 413)
            } catch is URLError {
                // The reset can win before the response is read; that's a legitimate failure.
            }
        }
    }

    @Test("A session keeps working after early responses")
    func sessionKeepsWorkingAfterEarlyResponses() async throws {
        let rejecting = try await LoopbackHTTPServer.rejectingEarly()
        defer { rejecting.stop() }
        let accepting = try await LoopbackHTTPServer.accepting()
        defer { accepting.stop() }
        let session = Support.session()

        for _ in 0..<3 {
            let (_, response) = try await upload(
                body: .onDisk(try Support.makeTempFile()),
                with: Support.request(to: rejecting.url),
                session: session,
                delegate: nil
            )
            #expect((response as? HTTPURLResponse)?.statusCode == 413)
        }

        let (_, response) = try await upload(
            body: .onDisk(try Support.makeTempFile()),
            with: Support.request(to: accepting.url),
            session: session,
            delegate: nil
        )
        #expect((response as? HTTPURLResponse)?.statusCode == 201)
    }
}

@Suite("MultipartFormContent length")
struct MultipartFormContentLengthTests {
    @Test("An in-memory body's length is its byte count")
    func inMemoryLength() {
        #expect(MultipartFormContent.inMemory(Data(count: 1234)).length == 1234)
    }

    @Test("An on-disk body's length is its file size")
    func onDiskLength() throws {
        let file = try UploadTestSupport.makeTempFile(size: 4321)
        defer { try? FileManager.default.removeItem(at: file) }
        #expect(MultipartFormContent.onDisk(file).length == 4321)
    }

    @Test("A missing file has no length")
    func missingFileLength() {
        let file = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        #expect(MultipartFormContent.onDisk(file).length == nil)
    }
}
#endif
