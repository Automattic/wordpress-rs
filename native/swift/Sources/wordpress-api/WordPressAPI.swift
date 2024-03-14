import Foundation
import wordpress_api_wrapper

#if os(Linux)
import FoundationNetworking
#endif

public struct WordPressAPI {
    private let urlSession: URLSession
    package let helper: WpApiHelperProtocol

    public init(urlSession: URLSession, baseUrl: URL, authenticationStrategy: WpAuthentication) {
#if WP_SUPPORT_BACKGROUND_URL_SESSION
        // We use URLSession APIs that accept completion block, which doesn't work with background URLSession.
        // See `URLSession.backgroundSession(configuration:)` in `URLSession+WordPressAPI.swift`.
        assert(
            urlSession.configuration.identifier == nil || urlSession.delegate is BackgroundURLSessionDelegate,
            "Background URLSession must use BackgroundURLSessionDelegate"
        )
#else
        assert(
            urlSession.configuration.identifier == nil,
            "Background URLSession are not supported"
        )
#endif
        self.urlSession = urlSession
        self.helper = WpApiHelper(siteUrl: baseUrl.absoluteString, authentication: authenticationStrategy)
    }

    package func perform(request: WpNetworkRequest) async throws -> WpNetworkResponse {
        try await self.urlSession.perform(request: request).get()
    }

    package func perform(request: WpNetworkRequest, callback: @escaping (Result<WpNetworkResponse, Error>) -> Void) {
        Task {
            do {
                let result = try await self.perform(request: request)
                callback(.success(result))
            } catch {
                callback(.failure(error))
            }
        }
    }
}

extension Result {
    @inlinable public func tryMap<NewSuccess>(
            _ transform: (Success) throws -> NewSuccess
    ) -> Result<NewSuccess, any Error> {
        switch self {
        case .success(let success):
            do {
                return .success(try transform(success))
            } catch let err {
                return .failure(err)
            }

        case .failure(let error): return .failure(error)
        }
    }
}

// Note: Everything below this line should be moved into the Rust layer
public extension WpAuthentication {
    init(username: String, password: String) {
        self.init(authToken: "\(username):\(password)".data(using: .utf8)!.base64EncodedString())
    }
}

extension RequestMethod {
    var rawValue: String {
        switch self {
        case .get: "GET"
        case .post: "POST"
        case .put: "PUT"
        case .delete: "DELETE"
        }
    }
}
