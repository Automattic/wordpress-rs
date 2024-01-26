import Foundation
import wordpress_api_wrapper

public struct WordPressAPI {

    private let urlSession: URLSession
    package let helper: WpApiHelperProtocol

    public init(urlSession: URLSession, baseUrl: URL, authenticationStategy: WpAuthentication) {
        self.urlSession = urlSession
        self.helper = WpApiHelper(url: baseUrl.absoluteString, authentication: authenticationStategy)
    }

    package func perform(request: WpNetworkRequest) async throws -> WpNetworkResponse {
        let (data, response) = try await self.urlSession.data(for: request.asURLRequest())

        return WpNetworkResponse(
            body: data,
            statusCode: response.httpStatusCode,
            headerMap: response.httpHeaders
        )
    }

    package func perform(request: WpNetworkRequest, callback: @escaping (Result<WpNetworkResponse, Error>) -> Void) {
        self.urlSession.dataTask(with: request.asURLRequest()) { data, response, error in
            if let error {
                callback(.failure(error))
                return
            }

            guard let data = data, let response = response else {
                abort() // TODO: We should have a custom error type here that represents an inability to parse whatever came back
            }

            callback(.success(WpNetworkResponse(
                body: data,
                statusCode: response.httpStatusCode,
                headerMap: response.httpHeaders
            )))
        }
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
    @inlinable public func tryMap<NewSuccess>(_ transform: (Success) throws -> NewSuccess) -> Result<NewSuccess, any Error> {
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

// TODO: Everything below this line should be moved into the Rust layer
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
