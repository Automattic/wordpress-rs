import Foundation
#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

#if os(Linux)
import FoundationNetworking
#endif

public struct WordPressAPI {

    enum Errors: Error {
        case unableToParseResponse
    }

    private let urlSession: URLSession
    package let requestBuilder: WpRequestBuilderProtocol

    public init(urlSession: URLSession, baseUrl: URL, authenticationStategy: WpAuthentication) throws {
        try self.init(
            urlSession: urlSession,
            baseUrl: baseUrl,
            authenticationStategy: authenticationStategy,
            executor: urlSession
        )
    }

    init(
        urlSession: URLSession,
        baseUrl: URL,
        authenticationStategy: WpAuthentication,
        executor: SafeRequestExecutor
    ) throws {
        self.urlSession = urlSession
        self.requestBuilder = try WpRequestBuilder(
            siteUrl: baseUrl.absoluteString,
            authentication: authenticationStategy,
            requestExecutor: executor
        )
    }

    public var users: UsersRequestExecutor {
        self.requestBuilder.users()
    }

    public var plugins: PluginsRequestExecutor {
        self.requestBuilder.plugins()
    }

    package func perform(request: WpNetworkRequest) async throws -> WpNetworkResponse {
        try await withCheckedThrowingContinuation { continuation in
            self.perform(request: request) { result in
                continuation.resume(with: result)
            }
        }
    }

    package func perform(
        request: WpNetworkRequest,
        callback: @escaping @Sendable (Result<WpNetworkResponse, Error>) -> Void
    ) {
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
        public static func extractLoginDetails(from url: URL) -> WpApiApplicationPasswordDetails? {
            return extractLoginDetailsFromUrl(url: url.asRestUrl())
        }
    }

    enum ParseError: Error {
        case invalidUrl
        case invalidHtml
    }
}

public extension WpNetworkHeaderMap {
    func toFlatMap() -> [String: String] {
        self.toMap().mapValues { $0.joined(separator: ",") }
    }
}

public extension WpNetworkRequest {
    func asURLRequest() -> URLRequest {
        let url = URL(string: self.url())!
        var request = URLRequest(url: url)
        request.httpMethod = self.method().rawValue
        request.allHTTPHeaderFields = self.headerMap().toFlatMap()
        request.httpBody = self.body()?.contents()
        return request
    }

    #if DEBUG
    func debugPrint() {
        print("\(method().rawValue) \(self.url())")
        for (name, value) in self.headerMap().toMap() {
            print("\(name): \(value)")
        }

        print("")

        if let bodyString = self.bodyAsString() {
            print(bodyString)
        }
    }
    #endif
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
            headerMap: try WpNetworkHeaderMap.fromMap(hashMap: response.httpHeaders)
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
        self = .authorizationHeader(token: "\(username):\(password)".data(using: .utf8)!.base64EncodedString())
    }
}

extension RequestMethod {
    var rawValue: String {
        switch self {
        case .get: "GET"
        case .post: "POST"
        case .put: "PUT"
        case .delete: "DELETE"
        case .head: "HEAD"
        }
    }
}

extension WpRestApiUrl {
    func asUrl() -> URL {
        guard let url = URL(string: stringValue) else {
            preconditionFailure("Invalid URL: \(stringValue)")
        }

        return url
    }
}

extension URL {
    func asRestUrl() -> WpRestApiUrl {
        WpRestApiUrl(stringValue: self.absoluteString)
    }
}
