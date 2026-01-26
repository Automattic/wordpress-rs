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

extension WpApiError {
    public var isCancellationError: Bool {
        if case .RequestExecutionFailed(statusCode: _, redirects: _, reason: .cancellationError) = self {
            return true
        }
        return false
    }
}

// MARK: - Enum initialization and unpacking
public extension CommentStatus {
    init(_ status: String) {
        self = commentStatusFromString(value: status)
    }

    var rawValue: String {
        self.asString()
    }
}

extension CommentStatus: ExpressibleByStringLiteral {
    public init(stringLiteral: String) {
        self.init(stringLiteral)
    }
}

public extension CommentType {
    init(_ type: String) {
        self = commentTypeFromString(value: type)
    }

    var rawValue: String {
        self.asString()
    }
}

extension CommentType: ExpressibleByStringLiteral {
    public init(stringLiteral: StringLiteralType) {
        self.init(stringLiteral)
    }
}

public extension UserRole {
    init(_ role: String) {
        self = userRoleFromString(value: role)
    }

    var rawValue: String {
        self.asString()
    }
}

extension UserRole: ExpressibleByStringLiteral {
    public init(stringLiteral: StringLiteralType) {
        self.init(stringLiteral)
    }
}

public extension UserCapability {
    init(_ capability: String) {
        self = userCapabilityFromString(value: capability)
    }

    var rawValue: String {
        self.asString()
    }
}

extension UserCapability: ExpressibleByStringLiteral {
    public init(stringLiteral: StringLiteralType) {
        self.init(stringLiteral)
    }
}

public extension PostType {
    init (_ type: String) {
        self = postTypeFromString(value: type)
    }

    var rawValue: String {
        self.asString()
    }
}

extension PostType: ExpressibleByStringLiteral {
    public init(stringLiteral: StringLiteralType) {
        self.init(stringLiteral)
    }
}

public extension PostTypeSupportsMap {
    func supports(_ feature: String) -> Bool {
        self.supports(feature: .custom(feature))
    }
}
