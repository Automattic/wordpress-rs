import Foundation
import wordpress_api_wrapper

#if os(Linux)
import FoundationNetworking
#endif

public struct WordPressAPI {

    enum Errors: Error {
        case unableToParseResponse
    }

    private let urlSession: URLSession
    package let helper: WpApiHelperProtocol

    public init(urlSession: URLSession, baseUrl: URL, authenticationStategy: WpAuthentication) {
        self.urlSession = urlSession
        self.helper = WpApiHelper(siteUrl: baseUrl.absoluteString, authentication: authenticationStategy)
    }

    package func perform(request: WpNetworkRequest) async throws -> WpNetworkResponse {
        try await withCheckedThrowingContinuation { continuation in
            self.perform(request: request) { result in
                continuation.resume(with: result)
            }
        }
    }

    package func perform(request: WpNetworkRequest, callback: @escaping (Result<WpNetworkResponse, Error>) -> Void) {
        let task = self.urlSession.dataTask(with: request.asURLRequest()) { data, response, error in
            if let error {
                callback(.failure(error))
                return
            }

            guard let data = data, let response = response else {
                callback(.failure(Errors.unableToParseResponse))
                return
            }

            do {
                let response = try WpNetworkResponse.from(data: data, response: response)
                callback(.success(response))
            } catch {
                callback(.failure(error))
            }
        }
        task.resume()
    }

    public struct Helpers {

        public static func parseUrl(string: String) throws -> URL {

            if let url = URL(string: string), url.scheme != nil {
                return url
            }

            if let url = URL(string: "http://" + string) {
                return url
            }

            if let url = URL(string: "http://" + string + "/") {
                return url
            }

            debugPrint("Invalid URL")

            throw ParseError.invalidUrl
        }

        static func findRestEndpoint(data: Data) throws -> URL {
            if let url = wordpress_api_wrapper.findRestEndpoint(bytes: data) {
                return url.asUrl()
            }

            throw ParseError.invalidHtml
        }

        public static func extractLoginDetails(from url: URL) -> WpapiApplicationPasswordDetails? {
            return wordpress_api_wrapper.extractLoginDetailsFromUrl(url: url.asRestUrl())
        }
    }

    enum ParseError: Error {
        case invalidUrl
        case invalidHtml
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

extension WpNetworkResponse {
    static func from(data: Data, response: URLResponse) throws -> WpNetworkResponse {
        guard let response = response as? HTTPURLResponse else {
            abort()
        }

        return WpNetworkResponse(
            body: data,
            statusCode: UInt16(response.statusCode),
            headerMap: response.httpHeaders
        )

    }
}

extension HTTPURLResponse {

    var httpHeaders: [String: String] {
        allHeaderFields.reduce(into: [String: String]()) {
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

extension WpNetworkRequest {
    init(method: RequestMethod, url: URL, headerMap: [String: String]? = nil) {
        self.init(method: method, url: url.absoluteString, headerMap: headerMap)
    }
}

extension WpRestApiurl {
    func asUrl() -> URL {
        guard let url = URL(string: stringValue) else {
            preconditionFailure("Invalid URL: \(stringValue)")
        }

        return url
    }
}

extension URL {
    func asRestUrl() -> WpRestApiurl {
        WpRestApiurl(stringValue: self.absoluteString)
    }
}
