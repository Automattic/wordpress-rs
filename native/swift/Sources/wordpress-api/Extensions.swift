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
        commentStatusToString(status: self)
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
        commentTypeToString(commentType: self)
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
        userRoleToString(role: self)
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
        userCapabilityToString(capability: self)
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
        postTypeToString(postType: self)
    }
}

extension PostType: ExpressibleByStringLiteral {
    public init(stringLiteral: StringLiteralType) {
        self.init(stringLiteral)
    }
}

extension Array where Element == TaxonomyTypeDetailsWithEditContext {
    public func serialize() throws -> Data {
        try serializeTaxonomyTypeDetailsWithEditContextList(value: self)
    }

    static public func deserialize(from data: Data) throws -> [TaxonomyTypeDetailsWithEditContext] {
        try deserializeTaxonomyTypeDetailsWithEditContextList(value: data)
    }
}
