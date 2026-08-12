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
}
