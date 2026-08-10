import Foundation
import Testing
@testable import WordPressAPI

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

class MultipartFormTests {

    /// An `InputStream` that yields `prefix`, then simulates a failure by returning
    /// `-1` with a `streamError` set — as happens when a file is deleted
    /// mid-serialization, or an external / file-provider / iCloud-evicted volume
    /// raises an I/O error.
    private final class FailingInputStream: InputStream {
        private let prefix: [UInt8]
        private var didDeliverPrefix = false
        private var failure: Error?

        init(prefix: [UInt8] = []) {
            self.prefix = prefix
            super.init(data: Data())
        }

        override func open() {}
        override func close() {}
        override var hasBytesAvailable: Bool { true }
        override var streamError: Error? { failure }

        override func read(_ buffer: UnsafeMutablePointer<UInt8>, maxLength len: Int) -> Int {
            if !didDeliverPrefix && !prefix.isEmpty {
                didDeliverPrefix = true
                let count = min(len, prefix.count)
                for index in 0..<count {
                    buffer[index] = prefix[index]
                }
                return count
            }
            failure = POSIXError(.EIO)
            return -1
        }
    }

    /// An `InputStream` whose `open()` fails, leaving it in `.error` with no bytes
    /// available — as happens when the backing file is deleted between the field's
    /// construction and serialization. The read loop is gated on `hasBytesAvailable`,
    /// so it never runs and the mid-read `-1` guard never fires; only the post-loop
    /// `streamStatus == .error` check can catch this. `read` is instrumented to prove
    /// the loop was skipped rather than entered.
    private final class FailedToOpenInputStream: InputStream {
        private(set) var didAttemptRead = false
        private var opened = false

        init() {
            super.init(data: Data())
        }

        override func open() { opened = true }
        override func close() {}
        override var streamStatus: Stream.Status { opened ? .error : .notOpen }
        override var hasBytesAvailable: Bool { false }
        override var streamError: Error? { opened ? POSIXError(.ENOENT) : nil }

        override func read(_ buffer: UnsafeMutablePointer<UInt8>, maxLength len: Int) -> Int {
            didAttemptRead = true
            return -1
        }
    }

    /// An `InputStream` that delivers `prefix` on its first `read` (a positive count, never
    /// -1), then reports failure only through `streamStatus`/`hasBytesAvailable` — modeling a
    /// stream that errors mid-body without a -1 read. The in-loop `-1` guard can't see this,
    /// so it exercises the post-loop `streamStatus == .error` check on a non-open-failure path.
    private final class PartialThenErrorInputStream: InputStream {
        private let prefix: [UInt8]
        private var didDeliverPrefix = false

        init(prefix: [UInt8]) {
            self.prefix = prefix
            super.init(data: Data())
        }

        override func open() {}
        override func close() {}
        override var hasBytesAvailable: Bool { !didDeliverPrefix }
        override var streamStatus: Stream.Status { didDeliverPrefix ? .error : .open }
        override var streamError: Error? { didDeliverPrefix ? POSIXError(.EIO) : nil }

        override func read(_ buffer: UnsafeMutablePointer<UInt8>, maxLength len: Int) -> Int {
            didDeliverPrefix = true
            let count = min(len, prefix.count)
            for index in 0..<count {
                buffer[index] = prefix[index]
            }
            return count
        }
    }

    /// Reads the serialized bytes back from either destination so a success test can assert
    /// the exact body on both the in-memory and temp-file (`forceWriteToFile`) paths. The
    /// on-disk path returns a temp file the caller owns; it's removed once read.
    private func serializedBody(_ content: MultipartFormContent) throws -> Data {
        switch content {
        case let .inMemory(data):
            return data
        case let .onDisk(url):
            defer { try? FileManager.default.removeItem(at: url) }
            return try Data(contentsOf: url)
        }
    }

    @Test
    func serializationThrowsWhenStreamFailsMidRead() throws {
        // A read that returns some bytes and then fails must not feed a negative count
        // into `Data`'s initializer, which would trap and crash `perform()`.
        let form: [MultipartFormField] = [
            MultipartFormField(text: "hello", name: "field1"),
            MultipartFormField(
                inputStream: FailingInputStream(prefix: Array("partial".utf8)),
                name: "file",
                filename: "photo.jpg",
                mimeType: "image/jpeg",
                bytes: 0
            )
        ]

        do {
            _ = try form.multipartFormDataStream(boundary: "boundary")
            Issue.record("Expected serialization to throw when the stream read fails")
        } catch let error as MultipartFormError {
            guard case .inaccessibleFile = error else {
                Issue.record("Expected .inaccessibleFile, got \(error)")
                return
            }
        }
    }

    @Test
    func serializationThrowsWhenStreamFailsImmediately() throws {
        // The first read fails (e.g. the file was deleted before any bytes were read).
        let form: [MultipartFormField] = [
            MultipartFormField(
                inputStream: FailingInputStream(),
                name: "file",
                filename: "photo.jpg",
                mimeType: "image/jpeg",
                bytes: 0
            )
        ]

        do {
            _ = try form.multipartFormDataStream(boundary: "boundary")
            Issue.record("Expected serialization to throw when the stream read fails")
        } catch let error as MultipartFormError {
            guard case .inaccessibleFile = error else {
                Issue.record("Expected .inaccessibleFile, got \(error)")
                return
            }
        }
    }

    @Test
    func serializationThrowsOnDiskPathWhenStreamFails() throws {
        // Forms over ~10 MB (or `forceWriteToFile`) serialize to a temp file instead of
        // memory — this is the path a large media upload actually takes. A mid-read
        // failure there must throw the same classified error rather than trap, and it
        // drives the temp-file cleanup branch in `multipartFormDataStream`.
        //
        // Asserting the temp file's removal is omitted deliberately: the path is an
        // internal UUID the test can't observe, and Swift Testing runs suites in
        // parallel, so diffing the shared temporary directory would be racy.
        let form: [MultipartFormField] = [
            MultipartFormField(text: "hello", name: "field1"),
            MultipartFormField(
                inputStream: FailingInputStream(prefix: Array("partial".utf8)),
                name: "file",
                filename: "photo.jpg",
                mimeType: "image/jpeg",
                bytes: 0
            )
        ]

        do {
            _ = try form.multipartFormDataStream(boundary: "boundary", forceWriteToFile: true)
            Issue.record("Expected serialization to throw when the stream read fails")
        } catch let error as MultipartFormError {
            guard case .inaccessibleFile = error else {
                Issue.record("Expected .inaccessibleFile, got \(error)")
                return
            }
        }
    }

    // Runs both destinations: `forceWriteToFile: true` is the temp-file path large media
    // uploads take — the real-world #1542 trigger — and it also drives the temp-file cleanup
    // branch when the guard throws; `false` is the in-memory path.
    @Test(arguments: [false, true])
    func serializationThrowsWhenStreamFailedToOpen(forceWriteToFile: Bool) throws {
        // The #1542 manifestation: `open()` failed, so `hasBytesAvailable` is already
        // false at loop entry and the read loop never runs. Without the post-loop
        // `streamStatus == .error` guard, serialization would emit the closing CRLF and
        // hand back a well-formed-but-empty part with no error — a silent truncation the
        // server can't detect. `didAttemptRead` asserts the loop was skipped, proving this
        // exercises the post-loop guard and not the mid-read `-1` path.
        let stream = FailedToOpenInputStream()
        let form: [MultipartFormField] = [
            MultipartFormField(text: "hello", name: "field1"),
            MultipartFormField(
                inputStream: stream,
                name: "file",
                filename: "photo.jpg",
                mimeType: "image/jpeg",
                bytes: 0
            )
        ]

        do {
            _ = try form.multipartFormDataStream(boundary: "boundary", forceWriteToFile: forceWriteToFile)
            Issue.record("Expected serialization to throw when the stream failed to open")
        } catch let error as MultipartFormError {
            guard case .inaccessibleFile = error else {
                Issue.record("Expected .inaccessibleFile, got \(error)")
                return
            }
        }

        #expect(stream.didAttemptRead == false)
    }

    @Test
    func serializationThrowsWhenStreamErrorsAfterPartialRead() throws {
        // A stream that returns some bytes and then surfaces failure via `streamStatus` rather
        // than a -1 read: the mid-read guard keys on `read() < 0`, so it never fires, and only
        // the post-loop `streamStatus == .error` check catches the truncation. This is why the
        // guard is placed after the loop rather than right after `open()`.
        let form: [MultipartFormField] = [
            MultipartFormField(
                inputStream: PartialThenErrorInputStream(prefix: Array("partial".utf8)),
                name: "file",
                filename: "photo.jpg",
                mimeType: "image/jpeg",
                bytes: 0
            )
        ]

        do {
            _ = try form.multipartFormDataStream(boundary: "boundary")
            Issue.record("Expected serialization to throw when the stream errors after a partial read")
        } catch let error as MultipartFormError {
            guard case .inaccessibleFile = error else {
                Issue.record("Expected .inaccessibleFile, got \(error)")
                return
            }
        }
    }

    @Test
    func serializationThrowsWhenFileDeletedBeforeSerialization() throws {
        // The end-to-end #1542 window through the production `fileAtPath:` initializer: a
        // file-backed field constructed while the file exists, then deleted before
        // serialization. The initializer reads the size and builds an unopened
        // `InputStream`; the later `open()` fails, so no bytes are ever read. Whether the
        // failure surfaces via the failed `open()` (post-loop guard) or a `read` returning
        // -1 (mid-read guard) can vary by platform, but either way the caller must observe
        // `.inaccessibleFile` rather than a silently-truncated upload.
        let tempURL = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try Data("some file content".utf8).write(to: tempURL)
        // The explicit delete below is the test action; this keeps the temp file from leaking
        // if the `fileAtPath:` initializer ever threw before we reach it.
        defer { try? FileManager.default.removeItem(at: tempURL) }

        let field = try MultipartFormField(
            fileAtPath: tempURL.path,
            name: "file",
            filename: "photo.jpg",
            mimeType: "image/jpeg"
        )
        try FileManager.default.removeItem(at: tempURL)

        let form: [MultipartFormField] = [MultipartFormField(text: "hello", name: "field1"), field]

        do {
            _ = try form.multipartFormDataStream(boundary: "boundary")
            Issue.record("Expected serialization to throw when the backing file was deleted")
        } catch let error as MultipartFormError {
            guard case .inaccessibleFile = error else {
                Issue.record("Expected .inaccessibleFile, got \(error)")
                return
            }
        }
    }

    @Test
    func serializationSucceedsForEmptyField() throws {
        // The post-loop `streamStatus == .error` guard must not false-fail a legitimately
        // empty (zero-byte) field. An empty `InputStream(data:)` never enters `.error`, so
        // the empty part serializes normally. This is why the fix checks stream status
        // rather than reconciling bytes-written against the field's advertised length,
        // which would reject empty parts and files that changed size after the attribute
        // read.
        let form: [MultipartFormField] = [
            MultipartFormField(text: "", name: "empty"),
            MultipartFormField(text: "value", name: "field2")
        ]

        let content = try form.multipartFormDataStream(boundary: "boundary")

        guard case let .inMemory(data) = content else {
            Issue.record("Expected in-memory content for a small form")
            return
        }

        let body = try #require(String(data: data, encoding: .utf8))
        let expected = [
            "--boundary\r\n",
            "Content-Disposition: form-data; name=\"empty\"\r\n",
            "\r\n",
            "\r\n",
            "--boundary\r\n",
            "Content-Disposition: form-data; name=\"field2\"\r\n",
            "\r\n",
            "value\r\n",
            "--boundary--\r\n"
        ]
        .joined()
        #expect(body == expected)
    }

    @Test
    func serializationSucceedsForValidFields() throws {
        let form: [MultipartFormField] = [
            MultipartFormField(text: "value1", name: "field1"),
            MultipartFormField(
                data: Data("binary-content".utf8),
                name: "file",
                filename: "data.bin",
                mimeType: "application/octet-stream"
            )
        ]

        let content = try form.multipartFormDataStream(boundary: "boundary")

        guard case let .inMemory(data) = content else {
            Issue.record("Expected in-memory content for a small form")
            return
        }

        let body = try #require(String(data: data, encoding: .utf8))

        // Assert the exact serialized bytes. A `contains`-based version stayed green even
        // with the mandatory blank line between a part's headers and its body removed
        // (each header already ends in its own CRLF), so it couldn't catch structurally
        // broken MIME. The exact match pins the `\r\n\r\n` separators, field ordering,
        // and the closing terminator.
        let expectedParts = [
            "--boundary\r\n",
            "Content-Disposition: form-data; name=\"field1\"\r\n",
            "\r\n",
            "value1\r\n",
            "--boundary\r\n",
            "Content-Disposition: form-data; name=\"file\"; filename=\"data.bin\"\r\n",
            "Content-Type: application/octet-stream\r\n",
            "\r\n",
            "binary-content\r\n",
            "--boundary--\r\n"
        ]
        #expect(body == expectedParts.joined())
    }

    // Runs both destinations: `false` is the in-memory path; `true` is the temp-file path
    // large media uploads take. Every other success test uses in-memory `text:`/`data:`
    // streams, so this is the only coverage of the production `fileAtPath:` →
    // `InputStream(fileAtPath:)` upload path serializing a valid, existing file *without*
    // throwing. That path is new surface for the post-loop `streamStatus == .error` guard;
    // a fully-read valid file ends `.atEnd`, never `.error`, so the guard stays silent.
    @Test(arguments: [false, true])
    func serializationSucceedsForFileBackedField(forceWriteToFile: Bool) throws {
        let tempURL = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try Data("file-content".utf8).write(to: tempURL)
        defer { try? FileManager.default.removeItem(at: tempURL) }

        let field = try MultipartFormField(
            fileAtPath: tempURL.path,
            name: "file",
            filename: "photo.jpg",
            mimeType: "image/jpeg"
        )
        let form: [MultipartFormField] = [MultipartFormField(text: "hello", name: "field1"), field]

        let content = try form.multipartFormDataStream(boundary: "boundary", forceWriteToFile: forceWriteToFile)
        let data = try serializedBody(content)

        let body = try #require(String(data: data, encoding: .utf8))
        let expected = [
            "--boundary\r\n",
            "Content-Disposition: form-data; name=\"field1\"\r\n",
            "\r\n",
            "hello\r\n",
            "--boundary\r\n",
            "Content-Disposition: form-data; name=\"file\"; filename=\"photo.jpg\"\r\n",
            "Content-Type: image/jpeg\r\n",
            "\r\n",
            "file-content\r\n",
            "--boundary--\r\n"
        ]
        .joined()
        #expect(body == expected)
    }

    @Test
    func serializationSucceedsForEmptyFileBackedField() throws {
        // Complements `serializationSucceedsForEmptyField` (in-memory) by pinning the empty
        // *file* branch the guard's comment specifically claims: a zero-byte file opens
        // successfully and reaches `.atEnd` (not `.error`) after its first `read` returns 0,
        // so the guard doesn't false-fail it. Exercised through the production `fileAtPath:`
        // initializer.
        let tempURL = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try Data().write(to: tempURL)
        defer { try? FileManager.default.removeItem(at: tempURL) }

        let field = try MultipartFormField(
            fileAtPath: tempURL.path,
            name: "file",
            filename: "empty.bin",
            mimeType: "application/octet-stream"
        )
        let form: [MultipartFormField] = [field, MultipartFormField(text: "value", name: "field2")]

        let content = try form.multipartFormDataStream(boundary: "boundary")
        let data = try serializedBody(content)

        let body = try #require(String(data: data, encoding: .utf8))
        let expected = [
            "--boundary\r\n",
            "Content-Disposition: form-data; name=\"file\"; filename=\"empty.bin\"\r\n",
            "Content-Type: application/octet-stream\r\n",
            "\r\n",
            "\r\n",
            "--boundary\r\n",
            "Content-Disposition: form-data; name=\"field2\"\r\n",
            "\r\n",
            "value\r\n",
            "--boundary--\r\n"
        ]
        .joined()
        #expect(body == expected)
    }
}

// Regression tests for #1540: a multipart body estimated over 10 MB (or built with
// `forceWriteToFile: true`) is serialized to a UUID-named temp file under
// `FileManager.default.temporaryDirectory` and uploaded via `URLSession.uploadTask(fromFile:)`.
// `URLSession` treats that file as caller-owned — it reads it for the duration of the transfer but
// never deletes it — so every large upload leaked a temp file until the OS eventually reclaimed it.
// The cleanup lives in `MultipartFormContent.removeTemporaryFileIfNeeded()`, invoked from
// `upload(...)` once the transfer finishes on any path.

@Suite("MultipartForm temp-file cleanup")
struct MultipartFormCleanupTests {

    @Test("removeTemporaryFileIfNeeded deletes the temp file an on-disk body created")
    func removesOnDiskTempFile() throws {
        // Use the production serialization path so the file under test is a real temp file that
        // `multipartFormDataStream` created, not one the test fabricated.
        let content = try [MultipartFormField(text: "hello", name: "field")]
            .multipartFormDataStream(boundary: "test-boundary", forceWriteToFile: true)

        guard case let .onDisk(url) = content else {
            Issue.record("forceWriteToFile: true should serialize to disk, got \(content)")
            return
        }
        #expect(FileManager.default.fileExists(atPath: url.path))

        content.removeTemporaryFileIfNeeded()
        #expect(!FileManager.default.fileExists(atPath: url.path))
    }

    @Test("removeTemporaryFileIfNeeded is a harmless no-op for an in-memory body")
    func inMemoryCleanupIsNoOp() {
        // An in-memory body owns no file; cleanup must simply do nothing rather than crash.
        MultipartFormContent.inMemory(Data("in memory".utf8)).removeTemporaryFileIfNeeded()
    }
}

@Suite("MultipartForm upload cleanup", .enabled(if: !isLinux()))
struct MultipartFormUploadTests {

    @Test("upload deletes the on-disk temp file after a successful transfer")
    func uploadRemovesTempFileOnSuccess() async throws {
        let file = try Self.makeTempFile()

        // `expectingFileAt:` makes the stub assert the temp file still exists while the transfer is
        // in flight (see `StubURLProtocol.startLoading`). Together with the post-return `!fileExists`
        // check below, this brackets the file's lifetime — it must outlive the transfer, then be
        // gone — so a regression that deletes it too early fails here, not just one that leaks it.
        let (data, response) = try await upload(
            body: .onDisk(file),
            with: Self.stubRequest(behavior: .success, expectingFileAt: file.path),
            session: Self.stubbedSession(),
            delegate: nil
        )

        #expect((response as? HTTPURLResponse)?.statusCode == 200)
        #expect(String(data: data, encoding: .utf8) == "ok")
        #expect(!FileManager.default.fileExists(atPath: file.path))
    }

    @Test("upload deletes the on-disk temp file after a failed transfer")
    func uploadRemovesTempFileOnFailure() async throws {
        let file = try Self.makeTempFile()

        await #expect(throws: (any Error).self) {
            try await upload(
                body: .onDisk(file),
                with: Self.stubRequest(behavior: .failure),
                session: Self.stubbedSession(),
                delegate: nil
            )
        }

        #expect(!FileManager.default.fileExists(atPath: file.path))
    }

    // MARK: - Helpers

    private static func makeTempFile() throws -> URL {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try Data("multipart body bytes".utf8).write(to: url)
        return url
    }

    private static func stubbedSession() -> URLSession {
        let config = URLSessionConfiguration.ephemeral
        config.protocolClasses = [StubURLProtocol.self]
        return URLSession(configuration: config)
    }

    private static func stubRequest(
        behavior: StubURLProtocol.Behavior,
        expectingFileAt expectedFilePath: String? = nil
    ) -> URLRequest {
        var request = URLRequest(url: URL(string: "https://example.com/upload")!)
        request.httpMethod = "POST"
        request.setValue(behavior.rawValue, forHTTPHeaderField: StubURLProtocol.behaviorHeader)
        if let expectedFilePath {
            request.setValue(expectedFilePath, forHTTPHeaderField: StubURLProtocol.expectedFileHeader)
        }
        return request
    }
}

/// A stateless `URLProtocol` that returns a canned response so `upload(...)` can be exercised
/// without real networking. The desired behavior is carried on the request via a header rather than
/// shared mutable state, so concurrently-running tests don't interfere with each other.
final class StubURLProtocol: URLProtocol {
    static let behaviorHeader = "X-Stub-Behavior"
    static let expectedFileHeader = "X-Stub-Expected-File"

    enum Behavior: String {
        case success
        case failure
    }

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }
    override func stopLoading() {}

    override func startLoading() {
        // A `.onDisk` upload's temp file must survive until the transfer completes; deleting it any
        // earlier — e.g. hoisting the cleanup out of `upload(...)`'s `defer` — would fail a real large
        // upload mid-read. `startLoading()` runs while the transfer is in flight, before we signal
        // completion below, so the file must still exist here. If a caller passed its path and it's
        // already gone, surface a distinct error so the upload test's assertions fail loudly instead
        // of a premature-delete regression slipping through a "gone afterward" check.
        let expectedFilePath = request.value(forHTTPHeaderField: Self.expectedFileHeader)
        if let expectedFilePath, !FileManager.default.fileExists(atPath: expectedFilePath) {
            client?.urlProtocol(self, didFailWithError: URLError(.fileDoesNotExist))
            return
        }

        let headerValue = request.value(forHTTPHeaderField: Self.behaviorHeader)
        let behavior = headerValue.flatMap(Behavior.init(rawValue:)) ?? .success

        switch behavior {
        case .success:
            let response = HTTPURLResponse(
                url: request.url!,
                statusCode: 200,
                httpVersion: "HTTP/1.1",
                headerFields: [:]
            )!
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: Data("ok".utf8))
            client?.urlProtocolDidFinishLoading(self)
        case .failure:
            client?.urlProtocol(self, didFailWithError: URLError(.notConnectedToInternet))
        }
    }
}
