import Foundation

import wordpress_api_wrapper

public struct WordPressAPI {

    private let urlSession: URLSession
    private let helper: WpApiHelperProtocol

    public init(urlSession: URLSession, baseUrl: URL, authenticationStategy: WpAuthentication) {
        self.urlSession = urlSession
        self.helper = WpApiHelper(url: baseUrl.absoluteString, authentication: authenticationStategy)
    }

    public func listPosts(params: PostListParams = PostListParams()) async throws -> PostListResponse {
        let request = self.helper.postListRequest(params: params)
        let response = try await perform(request: request)
        return try parsePostListResponse(response: response)
    }

    public func retrievePost(id: UInt32, params: PostRetrieveParams? = nil) async throws -> PostObject? {
        nil
    }

    public func listPosts(url: String) async throws -> PostListResponse {
        let request = self.helper.rawRequest(url: url)
        let response = try await perform(request: request)
        return try parsePostListResponse(response: response)
    }

    private func perform(request: WpNetworkRequest) async throws -> WpNetworkResponse {
        let (data, response) = try await self.urlSession.data(for: request.asURLRequest())

        return WpNetworkResponse(
            body: data,
            statusCode: response.httpStatusCode,
            headerMap: response.httpHeaders
        )
    }
}

public extension WpNetworkRequest {
    func asURLRequest() -> URLRequest {
        let url = URL(string: self.url)!
        var request = URLRequest(url: url)
        request.httpMethod = self.method.rawValue
        request.allHTTPHeaderFields = self.headerMap
        return request
    }
}

extension URLResponse {
    var httpStatusCode: UInt16 {
        UInt16((self as! HTTPURLResponse).statusCode)
    }

    var httpHeaders: [String: String] {
        (self as! HTTPURLResponse).allHeaderFields.reduce(into: [String: String]()) {
            guard
                let key = $1.key as? String,
                let value = $1.value as? String
            else {
                return
            }

            $0.updateValue(value, forKey: key)
        }
    }
}

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
