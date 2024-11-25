import Foundation

struct MultipartRequestBody {

    private let parts: [HttpPart]
    let boundaryString: String = "wordpress-rs-swift-boundary"

    private let boundaryMarker: Data = Data([0x2D, 0x2D])
    private let lineBreak: Data = Data([0x0D, 0x0A])

    var boundaryData: Data {
        Data(boundaryString.utf8)
    }

    init(parts: [HttpPart] = []) {
        self.parts = parts
    }

    func addPart(_ part: HttpPart) -> Self {
        var mutableParts = self.parts
        mutableParts.append(part)

        return MultipartRequestBody(parts: mutableParts)
    }

    func build() async throws -> URL {
        let filePath = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try "".write(to: filePath, atomically: true, encoding: .utf8) // Create an empty file with built-in `throw`
        let fileHandle = try FileHandle(forWritingTo: filePath)

        for part in parts {
            var data = Data()
            data.append(contentsOf: boundaryMarker)
            data.append(boundaryData)
            data.append(contentsOf: lineBreak)
            data.append(part.httpHeadersData)
            try fileHandle.write(contentsOf: data)

            for try await bodyData in part.readData() {
                try fileHandle.write(contentsOf: bodyData)
            }

            try fileHandle.write(contentsOf: lineBreak)
        }

        try fileHandle.write(contentsOf: boundaryMarker + boundaryData + boundaryMarker + lineBreak)

        try fileHandle.close()

        return filePath
    }
}

enum HttpPart {

    case formData(name: String, data: [String: String])
    case file(name: String, filePath: URL, mimeType: String)

    var httpHeaders: [String: String] {
        return switch self {
        case .formData(let name, _): [
            "Content-Disposition": "form-data; name=\"\(name)\""
        ]
        case .file(let name, let fileName, let mimeType): [
            "Content-Disposition": "form-data; name=\"\(name)\"; filename=\"\(fileName.lastPathComponent)\"",
            "Content-Type": mimeType
        ]
        }
    }

    var httpHeadersData: Data {
        Data(httpHeadersString.utf8)
    }

    var httpHeadersString: String {
        httpHeaders
            .sorted { $0.key < $1.key }
            .compactMap { "\($0): \($1)" }
            .joined(separator: "\r\n")
            .appending("\r\n\r\n")
    }

    func readData() -> AsyncThrowingStream<Data, Error> {
        switch self {
        case .formData(name: _, data: let data):
                return AsyncThrowingStream {
                    $0.yield(convertToFormData(data))
                    $0.finish()
                }
        case .file(_, let filePath, _):
                return AsyncThrowingStream {
                    do {
                        let fileHandle = try FileHandle(forReadingFrom: filePath)

                        let chunkSize = 4_096_000 // Copy the file in 4MB chunks

                        repeat {
                            let newData = fileHandle.readData(ofLength: chunkSize)
                            $0.yield(newData)

                            if newData.count < chunkSize {
                                break
                            }
                        } while(true)

                        $0.finish()
                    } catch {
                        $0.finish(throwing: error)
                    }
                }
        }
    }

    func convertToFormData(_ data: [String: String]) -> Data {
        Data(convertToFormString(data).utf8)
    }

    func convertToFormString(_ data: [String: String]) -> String {
        data
            .compactMap { (key: String, value: String) -> (String, String)? in
                guard
                    let newKey = escape(key),
                    let newValue = escape(value)
                else {
                    return nil
                }

                return (newKey, newValue)
            }
            .sorted { $0.0 < $1.0 }
            .compactMap { "\($0)=\($1)" }
            .joined(separator: "&")
    }

    func escape(_ string: String) -> String? {
        var allowedCharacters = CharacterSet.alphanumerics
        allowedCharacters.insert(charactersIn: "*-._ ")

        return string
            .addingPercentEncoding(withAllowedCharacters: allowedCharacters)?
            .replacingOccurrences(of: " ", with: "+")
    }
}
