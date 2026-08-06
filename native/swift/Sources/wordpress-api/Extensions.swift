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
            statusCode: UInt32(response.statusCode),
            responseHeaderMap: try WpNetworkHeaderMap.fromMap(hashMap: response.httpHeaders),
            requestUrl: request.url(),
            requestMethod: request.method(),
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
        if case .RequestExecutionFailed(
            statusCode: _,
            redirects: _,
            reason: .cancellationError,
            requestUrl: _,
            requestMethod: _
        ) = self {
            return true
        }
        return false
    }

    /// Whether the site could not be reached at all — the host did not resolve,
    /// refused the connection, or the URL was malformed.
    ///
    /// Distinct from ``isDeviceOffline``: this indicates a problem reaching
    /// *this particular site*, not a loss of device connectivity.
    public var isSiteUnreachable: Bool {
        executionErrorReason?.isSiteUnreachable ?? false
    }

    /// Whether the request failed because the device has no network connection.
    ///
    /// Distinct from ``isSiteUnreachable``: the site itself may be perfectly
    /// healthy.
    public var isDeviceOffline: Bool {
        executionErrorReason?.isDeviceOffline ?? false
    }

    /// The underlying reason when this is a request execution failure, otherwise `nil`.
    private var executionErrorReason: RequestExecutionErrorReason? {
        if case .RequestExecutionFailed(
            statusCode: _,
            redirects: _,
            reason: let reason,
            requestUrl: _,
            requestMethod: _
        ) = self {
            return reason
        }
        return nil
    }
}

extension RequestExecutionError {
    /// Whether the site could not be reached at all — the host did not resolve,
    /// refused the connection, or the URL was malformed.
    public var isSiteUnreachable: Bool {
        executionErrorReason?.isSiteUnreachable ?? false
    }

    /// Whether the request failed because the device has no network connection.
    public var isDeviceOffline: Bool {
        executionErrorReason?.isDeviceOffline ?? false
    }

    /// The underlying reason when this is a request execution failure, otherwise `nil`.
    private var executionErrorReason: RequestExecutionErrorReason? {
        if case .RequestExecutionFailed(
            statusCode: _,
            redirects: _,
            reason: let reason,
            requestUrl: _,
            requestMethod: _
        ) = self {
            return reason
        }
        return nil
    }
}

extension RequestExecutionErrorReason {
    /// Whether the site could not be reached at all — the host did not resolve,
    /// refused the connection, or the URL was malformed.
    ///
    /// Distinct from ``isDeviceOffline``: this indicates a problem reaching
    /// *this particular site*, not a loss of device connectivity.
    public var isSiteUnreachable: Bool {
        requestExecutionErrorReasonIsSiteUnreachable(reason: self)
    }

    /// Whether the request failed because the device has no network connection.
    ///
    /// Distinct from ``isSiteUnreachable``: the site itself may be perfectly
    /// healthy.
    public var isDeviceOffline: Bool {
        requestExecutionErrorReasonIsDeviceOffline(reason: self)
    }
}

// MARK: - Enum initialization and unpacking
public extension CommentStatus {
    init(_ status: String) {
        self = commentStatusFromString(value: status)
    }

    var rawValue: String {
        self.description
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
        self.description
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
        self.description
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
        self.description
    }
}

extension UserCapability: ExpressibleByStringLiteral {
    public init(stringLiteral: StringLiteralType) {
        self.init(stringLiteral)
    }
}

public extension PostType {
    init(_ type: String) {
        self = postTypeFromString(value: type)
    }

    var rawValue: String {
        self.description
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
