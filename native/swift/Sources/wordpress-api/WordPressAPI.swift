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
    package let requestBuilder: UniffiWpApiClient

    public init(urlSession: URLSession, baseUrl: ParsedUrl, authenticationStategy: WpAuthentication) {
        self.init(
            urlSession: urlSession,
            baseUrl: baseUrl,
            authenticationStategy: authenticationStategy,
            executor: urlSession
        )
    }

    init(
        urlSession: URLSession,
        baseUrl: ParsedUrl,
        authenticationStategy: WpAuthentication,
        executor: SafeRequestExecutor
    ) {
        self.urlSession = urlSession
        self.requestBuilder = UniffiWpApiClient(
            siteUrl: baseUrl,
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

    public var applicationPasswords: ApplicationPasswordsRequestExecutor {
        self.requestBuilder.applicationPasswords()
    }

    public var siteHealthTests: WpSiteHealthTestsRequestExecutor {
        self.requestBuilder.wpSiteHealthTests()
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

            // If the task is cancelled, we can save time/CPU/battery by skipping the parsing step
            if Task.isCancelled {
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
        public static func extractLoginDetails(from url: URL) throws -> WpApiApplicationPasswordDetails? {
            let parsedUrl = try ParsedUrl.from(url: url)
            return try extractLoginDetailsFromUrl(url: parsedUrl)
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

public extension ParsedUrl {
    static func from(url: URL) throws -> ParsedUrl {
        try parse(input: url.absoluteString)
    }
}
