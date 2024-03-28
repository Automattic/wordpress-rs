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
    package let helper: WpApiHelper

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

        public static func extractLoginDetails(from url: URL) -> WpapiApplicationPasswordDetails? {
            return wordpress_api_wrapper.extractLoginDetailsFromUrl(url: url.asRestUrl())
        }
    }

    enum ParseError: Error {
        case invalidUrl
        case invalidHtml
    }
}

public protocol Paginatable {
    static func parse(list: Data) -> Result<[Self], WpApiError>
}

public class Paginator<R: Paginatable> {
    let api: WordPressAPI
    let rust: wordpress_api_wrapper.Paginator

    public init(api: WordPressAPI, route: String, perPage: UInt32) {
        self.api = api
        self.rust = .init(apiHelper: api.helper, route: route, query: nil, perPage: perPage)
    }

    public func nextPage() async throws -> [R] {
        // TODO: concurrent calls should be queued.
        do {
            let request = try rust.nextPage()
            let response = try await api.perform(request: request)
            let jsonData = try rust.receive(response: response)
            return try R.parse(list: jsonData).get()
        } catch let rustError as wordpress_api_wrapper.PaginationError {
            switch rustError {
            case .ReachedEnd, .Unknown:
                return []
            case .ApiError(error: let error):
                throw error
            }
        }
    }
}

extension PostObject: Paginatable {
    public static func parse(list: Data) -> Result<[PostObject], WpApiError> {
        do {
            let result = try wordpress_api_wrapper.parsePostList(json: list)
            return .success(result)
        } catch let error as WpApiError {
            return .failure(error)
        } catch {
            fatalError("Unexpected error: \(error)")
        }
    }
}

extension WordPressAPI {
    public func list<R: Paginatable>(type: R.Type, perPage: UInt32) -> AsyncThrowingStream<[R], Error /* PaginationError */> {
        let stream: (AsyncThrowingStream<[R], Error>.Continuation) -> Void = { continuation in
            Task.detached {
                let paginator = Paginator<R>(
                    api: self,
                    route: "wp/v2/posts",
                    perPage: perPage
                )

                while true /* unless cancelled */ {
                    let result = try await paginator.nextPage()
                    if result.isEmpty {
                        continuation.finish()
                    } else {
                        continuation.yield(result)
                    }
                }
            }
        }

        return AsyncThrowingStream<[R], Error>([R].self, stream)
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
