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
    init(data: Data, request: NetworkRequestContent, response: URLResponse) throws {
        guard let response = response as? HTTPURLResponse else {
            preconditionFailure("We should never wind up here")
        }

        self = WpNetworkResponse(
            body: data,
            statusCode: UInt16(response.statusCode),
            responseHeaderMap: try WpNetworkHeaderMap.fromMap(hashMap: response.httpHeaders),
            requestUrl: request.url(),
            requestHeaderMap: request.headerMap()
        )
    }
}

extension MiddlewarePipeline {
    convenience init(middlewares: Middleware...) {
        self.init(middlewares: middlewares)
    }
}

public extension CommentStatus {
    init(status: String) {
        self = commentStatusFromString(value: status)
    }

    var status: String {
        commentStatusToString(status: self)
    }
}

public extension CommentType {
    init(type: String) {
        self = commentTypeFromString(value: type)
    }

    var type: String {
        commentTypeToString(commentType: self)
    }
}
