import Foundation

enum MultipartFormError: Swift.Error, LocalizedError {
    case inaccessibleFile(underlyingError: Error)
    case impossible

    var errorDescription: String? {
        switch self {
        case let .inaccessibleFile(underlyingError: underlyingError):
            return underlyingError.localizedDescription
        case .impossible:
            return "An unknown error occurred."
        }
    }
}

enum MultipartFormContent {
    case inMemory(Data)
    case onDisk(URL)

    func asInputStream() -> InputStream {
        switch self {
        case let .inMemory(data):
            return InputStream(data: data)
        case let .onDisk(url):
            precondition(url.isFileURL && FileManager.default.fileExists(atPath: url.path))
            return InputStream(fileAtPath: url.path)!
        }
    }
}

struct MultipartFormField {
    let name: String
    let filename: String?
    let mimeType: String?
    let bytes: UInt64

    fileprivate let inputStream: InputStream

    init(text: String, name: String, filename: String? = nil, mimeType: String? = nil) {
        self.init(data: text.data(using: .utf8)!, name: name, filename: filename, mimeType: mimeType)
    }

    init(data: Data, name: String, filename: String? = nil, mimeType: String? = nil) {
        self.inputStream = InputStream(data: data)
        self.name = name
        self.filename = filename
        self.bytes = UInt64(data.count)
        self.mimeType = mimeType
    }

    init(fileAtPath path: String, name: String, filename: String? = nil, mimeType: String? = nil) throws {
        let attrs: [FileAttributeKey: Any]
        do {
            attrs = try FileManager.default.attributesOfItem(atPath: path)
        } catch {
            throw MultipartFormError.inaccessibleFile(underlyingError: error)
        }

        guard let inputStream = InputStream(fileAtPath: path),
            let bytes = (attrs[FileAttributeKey.size] as? NSNumber)?.uint64Value
        else {
            // Given we can successfully read the file attributes, the above calls should never fail.
            throw MultipartFormError.impossible
        }

        self.inputStream = inputStream
        self.name = name
        self.filename = filename ?? path.split(separator: "/").last.flatMap({ String($0) })
        self.bytes = bytes
        self.mimeType = mimeType
    }

    /// Creates a field backed by an arbitrary `InputStream`.
    ///
    /// Exposed for tests that need to exercise stream-read failures (`read`
    /// returning `-1`); production code uses the `text:`, `data:`, and
    /// `fileAtPath:` initializers above. `bytes` has no default so a caller can't
    /// silently under-estimate a large stream into the in-memory serialization path.
    init(inputStream: InputStream, name: String, filename: String? = nil, mimeType: String? = nil, bytes: UInt64) {
        self.inputStream = inputStream
        self.name = name
        self.filename = filename
        self.mimeType = mimeType
        self.bytes = bytes
    }
}

extension Array where Element == MultipartFormField {
    private func multipartFormDestination(
        forceWriteToFile: Bool
    ) throws -> (outputStream: OutputStream, tempFilePath: String?) {
        let dest: OutputStream
        let tempFilePath: String?

        // Build the form data in memory if the content is estimated to be less than 10 MB.
        // Otherwise, use a temporary file.
        let thresholdBytesForUsingTmpFile = 10_000_000
        let estimatedFormDataBytes = reduce(0) { $0 + $1.bytes }
        if forceWriteToFile || estimatedFormDataBytes > thresholdBytesForUsingTmpFile {
            let tempFile = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString).path
            guard let stream = OutputStream(toFileAtPath: tempFile, append: false) else {
                // This error should never occurr, because the `tempFile` is in a temporary directory
                // and is guranteed to be writable.
                throw MultipartFormError.impossible
            }
            dest = stream
            tempFilePath = tempFile
        } else {
            dest = OutputStream.toMemory()
            tempFilePath = nil
        }

        return (dest, tempFilePath)
    }

    func multipartFormDataStream(boundary: String, forceWriteToFile: Bool = false) throws -> MultipartFormContent {
        guard !isEmpty else {
            return .inMemory(Data())
        }

        let (dest, tempFilePath) = try multipartFormDestination(forceWriteToFile: forceWriteToFile)

        // Build the form content
        do {
            dest.open()
            defer { dest.close() }

            try writeMultipartFormData(destination: dest, boundary: boundary)
        } catch {
            // Serialization failed partway through (e.g. a stream read error). Don't
            // leave a half-written temp file behind.
            if let tempFilePath {
                try? FileManager.default.removeItem(atPath: tempFilePath)
            }
            throw error
        }

        // Return the result as `InputStream`
        if let tempFilePath {
            return .onDisk(URL(fileURLWithPath: tempFilePath))
        }

        if let data = dest.property(forKey: .dataWrittenToMemoryStreamKey) as? Data {
            return .inMemory(data)
        }

        throw MultipartFormError.impossible
    }

    private func writeMultipartFormData(destination dest: OutputStream, boundary: String) throws {
        for field in self {
            dest.writeMultipartForm(boundary: boundary, isEnd: false)

            // Write headers
            var disposition = ["form-data", "name=\"\(field.name)\""]
            if let filename = field.filename {
                disposition += ["filename=\"\(filename)\""]
            }
            dest.writeMultipartFormHeader(name: "Content-Disposition", value: disposition.joined(separator: "; "))

            if let mimeType = field.mimeType {
                dest.writeMultipartFormHeader(name: "Content-Type", value: mimeType)
            }

            // Write a linebreak between header and content
            dest.writeMultipartFormLineBreak()

            // Write content
            field.inputStream.open()
            defer {
                field.inputStream.close()
            }
            let maxLength = 1024
            var buffer = [UInt8](repeating: 0, count: maxLength)
            while field.inputStream.hasBytesAvailable {
                let bytesRead = field.inputStream.read(&buffer, maxLength: maxLength)

                // `read` returns 0 at the end of the stream and -1 on failure — for
                // example when the file was deleted after the field was constructed, or
                // a mid-read I/O error occurs on an external / file-provider / iCloud-
                // evicted volume. A negative count traps in
                // `Data(bytesNoCopy:count:deallocator:)`, so abort serialization with a
                // classified error rather than crashing.
                if bytesRead < 0 {
                    throw MultipartFormError.inaccessibleFile(
                        underlyingError: field.inputStream.streamError ?? POSIXError(.EIO)
                    )
                }
                if bytesRead == 0 {
                    break
                }

                dest.write(data: Data(bytesNoCopy: &buffer, count: bytesRead, deallocator: .none))
            }

            // The loop above is gated on `hasBytesAvailable`, which is already false when
            // `open()` failed — e.g. the file was deleted between the field's construction
            // and here, leaving the stream in `.error`. The body then never runs, no `read`
            // ever returns -1, and the mid-read guard above can't fire. Without this check
            // we'd emit the closing CRLF below and serialize a well-formed-but-empty part;
            // multipart/form-data carries no per-part length, so the server couldn't detect
            // the truncation and a should-fail upload would become a silently-wrong one. A
            // legitimately empty field leaves the stream `.open` (empty in-memory data) or
            // `.atEnd` (empty file), never `.error`, so this doesn't false-fail zero-byte
            // parts. It intentionally does not cover a file that *shrinks* between the size
            // read and serialization — that surfaces as a clean `read() == 0`/`.atEnd`, and
            // catching it would need the bytes-written reconciliation the issue rejected.
            if field.inputStream.streamStatus == .error {
                throw MultipartFormError.inaccessibleFile(
                    underlyingError: field.inputStream.streamError ?? POSIXError(.EIO)
                )
            }

            dest.writeMultipartFormLineBreak()
        }

        dest.writeMultipartForm(boundary: boundary, isEnd: true)
    }
}

private let multipartFormDataLineBreak = "\r\n"
private extension OutputStream {
    func write(data: Data) {
        let count = data.count
        guard count > 0 else { return }

        _ = data.withUnsafeBytes { (ptr: UnsafeRawBufferPointer) in
            write(ptr.bindMemory(to: UInt8.self).baseAddress!, maxLength: count)
        }
    }

    func writeMultipartForm(lineContent: String) {
        write(data: "\(lineContent)\(multipartFormDataLineBreak)".data(using: .utf8)!)
    }

    func writeMultipartFormLineBreak() {
        write(data: multipartFormDataLineBreak.data(using: .utf8)!)
    }

    func writeMultipartFormHeader(name: String, value: String) {
        writeMultipartForm(lineContent: "\(name): \(value)")
    }

    func writeMultipartForm(boundary: String, isEnd: Bool) {
        if isEnd {
            writeMultipartForm(lineContent: "--\(boundary)--")
        } else {
            writeMultipartForm(lineContent: "--\(boundary)")
        }
    }
}
