import Foundation
import Testing
@testable import WordPressAPI

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
    func inaccessibleFileErrorCarriesSourcePathForFileField() throws {
        // A file-backed field that fails mid-read carries its source path, so the executor
        // can name the file. `sourcePath` stands in for the path a `fileAtPath:` field retains.
        let path = "/tmp/uploads/photo.jpg"
        let form: [MultipartFormField] = [
            MultipartFormField(
                inputStream: FailingInputStream(prefix: Array("partial".utf8)),
                name: "file",
                filename: "photo.jpg",
                mimeType: "image/jpeg",
                bytes: 0,
                sourcePath: path
            )
        ]

        do {
            _ = try form.multipartFormDataStream(boundary: "boundary")
            Issue.record("Expected serialization to throw when the stream read fails")
        } catch let error as MultipartFormError {
            guard case let .inaccessibleFile(_, filePath) = error else {
                Issue.record("Expected .inaccessibleFile, got \(error)")
                return
            }
            #expect(filePath == path)
        }
    }

    @Test
    func inaccessibleFileErrorHasNoPathForInMemoryField() throws {
        // An in-memory field (no backing file) that fails carries a `nil` path, so the
        // executor's `filePath?` guard doesn't misclassify it as `MediaFileUnreadable`
        // and it correctly falls through to `.genericError`.
        let form: [MultipartFormField] = [
            MultipartFormField(
                inputStream: FailingInputStream(),
                name: "field1",
                bytes: 0
            )
        ]

        do {
            _ = try form.multipartFormDataStream(boundary: "boundary")
            Issue.record("Expected serialization to throw when the stream read fails")
        } catch let error as MultipartFormError {
            guard case let .inaccessibleFile(_, filePath) = error else {
                Issue.record("Expected .inaccessibleFile, got \(error)")
                return
            }
            #expect(filePath == nil)
        }
    }

    @Test
    func inaccessibleFileWithPathMapsToMediaFileUnreadable() throws {
        // The executor maps a mid-read failure that carries a source path to the dedicated
        // `.MediaFileUnreadable`, naming the file.
        let mapped = try #require(
            MultipartFormError.inaccessibleFile(underlyingError: POSIXError(.EIO), filePath: "/tmp/uploads/photo.jpg")
                .asRequestExecutionError
        )
        guard case let .MediaFileUnreadable(filePath) = mapped else {
            Issue.record("Expected .MediaFileUnreadable, got \(mapped)")
            return
        }
        #expect(filePath == "/tmp/uploads/photo.jpg")
    }

    @Test
    func inaccessibleFileWithoutPathStaysGeneric() throws {
        // A serialization failure with no backing file (nil path) — or `.impossible` — has
        // nothing to classify, so it maps to `nil` and the executor lets it fall through to
        // `.genericError`. This pins the `filePath?` guard: relaxing it to accept a nil path
        // would misclassify a non-file serialization failure as `MediaFileUnreadable`.
        #expect(
            MultipartFormError.inaccessibleFile(underlyingError: POSIXError(.EIO), filePath: nil)
                .asRequestExecutionError == nil
        )
        #expect(MultipartFormError.impossible.asRequestExecutionError == nil)
    }

    @Test
    func fileFieldRetainsFullSourcePath() throws {
        // The production `fileAtPath:` init must retain the full source path (not just the
        // basename in `filename`); a regression to nil would send every real upload's mid-read
        // failure to a path-less `.genericError`, which the stream-layer tests wouldn't catch.
        let path = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString + ".jpg").path
        #expect(FileManager.default.createFile(atPath: path, contents: Data("x".utf8)))
        defer { try? FileManager.default.removeItem(atPath: path) }

        let field = try MultipartFormField(fileAtPath: path, name: "file")
        #expect(field.sourcePath == path)

        // In-memory fields have no backing file, so they carry no source path.
        #expect(MultipartFormField(text: "hi", name: "field").sourcePath == nil)
        #expect(MultipartFormField(data: Data("d".utf8), name: "field").sourcePath == nil)
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
