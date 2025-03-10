import Foundation
@preconcurrency import WordPressAPIInternal

#if os(Linux)
import FoundationNetworking
#endif

public actor WordPressAPI {

    enum Errors: Error {
        case unableToParseResponse
    }

    private let internalExecutor: SafeRequestExecutor
    package let requestBuilder: UniffiWpApiClient

    public init(urlSession: URLSession, baseUrl: ParsedUrl, authenticationStategy: WpAuthentication) {
        self.init(
            baseUrl: baseUrl,
            authenticationStategy: authenticationStategy,
            executor: WpRequestExecutor(urlSession: urlSession)
        )
    }

    init(
        baseUrl: ParsedUrl,
        authenticationStategy: WpAuthentication,
        executor: SafeRequestExecutor
    ) {
        self.internalExecutor = executor

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

    public var postTypes: PostTypesRequestExecutor {
        self.requestBuilder.postTypes()
    }

    public var posts: PostsRequestExecutor {
        self.requestBuilder.posts()
    }

    public var media: MediaRequestExecutor {
        self.requestBuilder.media()
    }

    public var siteSettings: SiteSettingsRequestExecutor {
        self.requestBuilder.siteSettings()
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
        request.allHTTPHeaderFields?["X-REQUEST-ID"] = self.requestId()
        request.httpBody = self.body()?.contents()
        request.timeoutInterval = 10
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
