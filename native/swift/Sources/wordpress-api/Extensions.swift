import Foundation
import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

public extension MiddlewarePipeline {
    static var `default`: MiddlewarePipeline {
        MiddlewarePipeline(middlewares: [])
    }
}

extension WpNetworkResponse {
    init(data: Data, request: WpNetworkRequest, response: HTTPURLResponse) throws {
        self = WpNetworkResponse(
            body: data,
            statusCode: UInt16(response.statusCode),
            responseHeaderMap: try WpNetworkHeaderMap.fromMap(hashMap: response.httpHeaders),
            requestUrl: request.url(),
            requestHeaderMap: request.headerMap()
        )
    }

    init(mediaUploadRequest: MediaUploadRequest, data: Data, response: HTTPURLResponse) throws {
        self = WpNetworkResponse(
            body: data,
            statusCode: UInt16(response.statusCode),
            responseHeaderMap: try WpNetworkHeaderMap.fromMap(hashMap: response.httpHeaders),
            requestUrl: mediaUploadRequest.url(),
            requestHeaderMap: mediaUploadRequest.headerMap()
        )
    }
}

extension MiddlewarePipeline {
    convenience init(middlewares: Middleware...) {
        self.init(middlewares: middlewares)
    }
}

extension MediaUploadRequest {

    private var filename: String {
        filePath.lastPathComponent
    }

    private var filePath: URL {
        URL(fileURLWithPath: self.filePath())
    }

    func asUrlRequest() async throws -> URLRequest {
        let multipartRequestBody = try await MultipartRequestBody(parts: [
            HttpPart.formData(data: self.mediaParams()),
            HttpPart.file(name: filename, filePath: filePath, mimeType: self.fileContentType())
        ]).build()

        var fileRequest = try URLRequest(url: self.url().asUrl())
        fileRequest.httpMethod = self.method().rawValue
        fileRequest.httpBodyStream = InputStream(url: filePath)

        fileRequest.setHeaders(self.headerMap().toFlatMap())
        fileRequest.setValue("attachment; filename=test-1.jpg", forHTTPHeaderField: "Content-Disposition")
        if let fileSize = try? self.calculateFileSize(for: filePath) {
            fileRequest.setValue(String(fileSize), forHTTPHeaderField: "Content-Length")
        }
        fileRequest.setValue("foo", forHTTPHeaderField: "X-REQUEST-ID") // TODO: There should be a requestID for this

        return fileRequest
    }

    private func calculateFileSize(for url: URL) throws -> Int? {
        try url.resourceValues(forKeys: [.fileSizeKey]).fileSize
    }
}

enum WpEndpointUrlError: Error {
    case invalidUrlString
}

extension URLRequest {
    mutating func setHeaders(_ headers: [String: String]) {
        for (key, value) in headers {
            setValue(value, forHTTPHeaderField: key)
        }
    }
}

extension WpEndpointUrl {
    func asUrl() throws -> URL {
        guard let url = URL(string: self) else {
            throw WpEndpointUrlError.invalidUrlString
        }

        return url
    }
}
