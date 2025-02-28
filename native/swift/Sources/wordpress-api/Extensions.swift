import Foundation
import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

public extension MiddlewarePipeline {
    static var `default`: MiddlewarePipeline {
        defaultMiddlewarePipeline()
    }
}

extension WpNetworkResponse {
    init(data: Data, response: URLResponse) throws {
        guard let response = response as? HTTPURLResponse else {
            preconditionFailure("We should never wind up here")
        }

        self = WpNetworkResponse(
            body: data,
            statusCode: UInt16(response.statusCode),
            headerMap: try WpNetworkHeaderMap.fromMap(hashMap: response.httpHeaders)
        )
    }
}
