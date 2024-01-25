import Foundation

import wordpress_api_wrapper

public struct WordPressAPI {

    private let urlSession: URLSession
    private let helper: WpApiHelperProtocol

    public init(urlSession: URLSession, baseUrl: URL, authenticationStategy: WpAuthentication) {
        self.urlSession = urlSession
        self.helper = WpApiHelper(url: baseUrl.absoluteString, authentication: authenticationStategy)
    }

    public func listPosts(params: PostListParams) async throws -> PostListResponse {
        let request = self.helper.postListRequest()
        let response = try await perform(request: request)
        return try parsePostListResponse(response: response)
    }

    public func retrievePost(id: UInt32, params: PostRetrieveParams? = nil) async throws -> PostObject? {
        nil
    }

    private func perform(request: WpNetworkRequest) async throws -> WpNetworkResponse {
        let (data, rawResponse) = try await self.urlSession.data(for: request.asURLRequest())

        return WpNetworkResponse(
            body: data,
            statusCode: UInt16((rawResponse as! HTTPURLResponse).statusCode)
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
