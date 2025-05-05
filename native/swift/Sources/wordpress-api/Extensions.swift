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

    init(mediaUploadRequest: MediaUploadRequest, response: HTTPURLResponse) throws {
        self = WpNetworkResponse(
            body: Data(),
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
            HttpPart.file(name: filename, filePath: filePath, mimeType: self.fileContentType())
        ]).build()

        var request = try URLRequest(url: self.url().asUrl())
        request.allHTTPHeaderFields = self.headerMap().toFlatMap()
        request.httpBodyStream = InputStream(url: multipartRequestBody)

        return request
    }
}

enum WpEndpointUrlError: Error {
    case invalidUrlString
}

extension WpEndpointUrl {
    func asUrl() throws -> URL {
        guard let url = URL(string: self) else {
            throw WpEndpointUrlError.invalidUrlString
        }

        return url
    }
}
