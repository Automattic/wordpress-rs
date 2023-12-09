import Foundation

import wordpress_api_wrapper

public struct WordPressAPI {

    private let urlSession: URLSession
    private let authenticationStrategy: AuthenticationStrategy
    private let customNetworking: CustomNetworking

    public init(urlSession: URLSession, authenticationStategy: AuthenticationStrategy) {
        self.urlSession = urlSession
        self.authenticationStrategy = authenticationStategy

        self.customNetworking = CustomNetworking(urlSession: urlSession, authentication: authenticationStategy)
    }

    public func listPosts(params: PostListParams) async throws -> [PostObject] {
        let request = URLRequest(params, authenticationStrategy: authenticationStrategy)
        let (data, response) = try await perform(request: request)

        return []
    }

    public func retrievePost(id: UInt32, params: PostRetrieveParams? = nil) async throws -> PostObject? {
        let api = wpApiWithCustomNetworking(
            authentication: authenticationStrategy.toAuthentication(),
            networkingInterface: customNetworking
        )
        let response = api.retrievePost(postId: id, params: params)
        return response.post
    }

    public func createPost(params: PostCreateParams) async throws -> PostObject {
        let request = try URLRequest(params, authenticationStrategy: authenticationStrategy)
        let (data, response) = try await perform(request: request)
        return PostObject(id: 1, title: "foo", content: "bar")
    }

    private func perform(request: URLRequest) async throws -> (Data, HTTPURLResponse) {
        let (data, response) = try await urlSession.data(for: request)
        let urlRespone = response as! HTTPURLResponse

        if urlRespone.statusCode >= 400 {
            abort()
        }

        return (data, urlRespone)
    }

    public func combineStrings(_ lhs: String, _ rhs: String) -> String {
        wordpress_api_wrapper.combineStrings(a: lhs, b: rhs)
    }
}

public class CustomNetworking: WpNetworkingInterface {

    private let urlSession: URLSession
    private let authentication: AuthenticationStrategy

    init(urlSession: URLSession, authentication: AuthenticationStrategy) {
        self.urlSession = urlSession
        self.authentication = authentication
    }

    public func request(request: wordpress_api_wrapper.WpNetworkRequest) -> wordpress_api_wrapper.WpNetworkResponse {
        WpNetworkResponse()
    }
}

extension URLRequest {
    init(_ request: HttpGetRequest, authenticationStrategy: AuthenticationStrategy) {
        let url = URL(string: "https://public-api.wordpress.com")!
            ._appending(queryParams: request.asQueryParams())

        self.init(url: url)
        self.allHTTPHeaderFields = authenticationStrategy.toHttpHeaders()
        self.httpMethod = request.httpVerb.rawValue
    }

    init(_ request: HttpPostRequest, authenticationStrategy: AuthenticationStrategy) throws {
        let url = URL(string: "https://public-api.wordpress.com")!

        self.init(url: url)
        self.allHTTPHeaderFields = authenticationStrategy.toHttpHeaders()
        self.httpBody = try request.asHttpBody()
        self.httpMethod = request.httpVerb.rawValue
    }
}

extension URL {
    func _appending(queryParams: [URLQueryItem]) -> URL {
        var url = self
        if #available(macOS 13.0, *) {
            url.append(queryItems: queryParams)
        } else {
            var components = URLComponents(string: url.absoluteString)
            var queryItems = components?.queryItems ?? []
            queryItems.append(contentsOf: queryParams)
            components!.queryItems = queryItems
            return components!.url!
        }
        return url
    }
}

extension URLRequest {

    func appending(httpBody: Data) -> URLRequest {
        var mutableCopy = self
        mutableCopy.httpBody = httpBody
        return mutableCopy
    }
}

// Everything below this library should be pushed down into the shared layer

public enum AuthenticationStrategy: AuthenticationConvertable {

    /// Use a WordPress.org REST API Application Password
    ///
    /// See:  https://make.wordpress.org/core/2020/11/05/application-passwords-integration-guide/
    ///
    case applicationPassword(String)

    /// Use an OAuth token to authenticate – getting that token is external to this library
    ///
    case oauthToken(String)

    /// Use HTTP basic authentication (as with https://github.com/WP-API/Basic-Auth)
    ///
    case httpBasic(username: String, password: String)

    /// Use a set of cookie-provided key:value pairs to authenticate
    ///
    case cookie([String:String])

    /// Allow setting arbitrary key-value pairs for authentication approaches we haven't thought of yet
    ///
    case custom([String: String])

    public func toHttpHeaders() -> [String: String] {
        switch self {
        case .applicationPassword(let password):
            return ["Authorization": "Bearer " + password]
        case .oauthToken(let token):
            return ["Authorization": "Bearer " + token]
        case .httpBasic(let username, let password):
            let data = (username + ":" + password).data(using: .utf8)
            return ["Authorization": "Basic " + (data?.base64EncodedString() ?? "")]
        case .cookie(let cookie):
            return ["Cookie": "\(cookie)"] // TODO: This won't work
        case .custom(let headers):
            return headers
        }
    }

    public func toAuthentication() -> wordpress_api_wrapper.WpAuthentication {
        WpAuthentication(authToken: toHttpHeaders().first!.value)
    }
}

public protocol AuthenticationConvertable {
    func toHttpHeaders() -> [String: String]
    func toAuthentication() -> WpAuthentication
}

public protocol QueryParamConvertable {
    func asQueryParams() -> [URLQueryItem]
}

public protocol HttpBodyConvertable {
    func asHttpBody() throws -> Data
}

public enum HttpVerb: String {
    case GET
    case POST
}

public protocol HttpVerbConvertable {
    var httpVerb: HttpVerb { get }
}

typealias HttpGetRequest = HttpVerbConvertable & QueryParamConvertable
typealias HttpPostRequest = HttpVerbConvertable & HttpBodyConvertable

extension PostListParams: HttpVerbConvertable, QueryParamConvertable {

    public var httpVerb: HttpVerb { .GET }

    public func asQueryParams() -> [URLQueryItem] {

        var params = [URLQueryItem]()

        if let page = self.page {
            params.append(URLQueryItem(name: "page", value: String(page)))
        }

        if let perPage = self.perPage {
            params.append(URLQueryItem(name: "perPage", value: String(perPage)))
        }

        return params
    }
}

extension PostRetrieveParams: HttpVerbConvertable, QueryParamConvertable {
    public var httpVerb: HttpVerb { .GET }

    public func asQueryParams() -> [URLQueryItem] {
        var params = [URLQueryItem]()

        if let password = self.password {
            params.append(URLQueryItem(name: "password", value: String(password)))
        }

        return params
    }
}

extension PostCreateParams: HttpVerbConvertable, HttpBodyConvertable {

    public var httpVerb: HttpVerb { .POST }

    public func asHttpBody() throws -> Data {
        try JSONEncoder().encode([
            "title": self.title,
            "content": self.content
        ])
    }
}
