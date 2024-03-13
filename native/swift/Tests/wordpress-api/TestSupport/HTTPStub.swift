import Foundation
import XCTest
import FlyingFox

#if os(Linux)
import FoundationNetworking
#endif

final class HTTPStub {
    let serverURL: URL

    private let server: HTTPServer
    private var stubs: [(condition: RequestFilter, response: ResponseBlock)]

    fileprivate init() async throws {
        let port: UInt16 = (8000...9999).randomElement()!
        let server = HTTPServer(port: port)
        Task.detached { try await server.start() }
        try await server.waitUntilListening()

        self.server = server
        self.serverURL = URL(string: "http://localhost:\(port)")!
        self.stubs = []

        let handler = { @Sendable [weak self] (request: HTTPRequest) async throws -> HTTPResponse  in
            guard let self else { return HTTPResponse(statusCode: .serviceUnavailable) }

            let urlRequest = try await request.asURLRequest(serverURL: serverURL)
            guard let responseBlock = self.stubs.first(where: { condition, _ in condition(urlRequest) })?.response else {
                return HTTPResponse(statusCode: .notFound)
            }

            return try await responseBlock(urlRequest).response(for: urlRequest)
        }
        await server.appendRoute("*", handler: handler)
    }

    func terminate() async {
        await server.stop()
    }
}

extension HTTPRequest {

    func asURLRequest(serverURL: URL) async throws -> URLRequest {
        var components = try XCTUnwrap(URLComponents(url: serverURL, resolvingAgainstBaseURL: true))
        components.path = path
        components.queryItems = query.map {
            URLQueryItem(name: $0.name, value: $0.value)
        }
        let url = try XCTUnwrap(components.url)

        var request = URLRequest(url: url)
        request.httpMethod = method.rawValue
        for (name, value) in headers {
            request.setValue(value, forHTTPHeaderField: name.rawValue)
        }
        request.httpBody = try await bodyData
        return request
    }

}

extension XCTestCase {

    func launchHTTPStub() async throws -> HTTPStub {
        for _ in 1...5 {
            do {
                let stub = try await HTTPStub()
                addTeardownBlock {
                    await stub.terminate()
                }
                return stub
            } catch {
                print("Failed to create an HTTP server: \(error)")
            }
        }

        // Final attempt
        return try await HTTPStub()
    }

}

struct HTTPStubsResponse {
    var fileURL: URL?
    var data: Data?
    var statusCode: Int?
    var headers: [String: String]?

    var responseTime: TimeInterval?

    func response(for request: URLRequest) async throws -> HTTPResponse {
        if let responseTime {
            let milliseconds: UInt64 = UInt64(1_000 * responseTime)
            try await Task.sleep(nanoseconds: milliseconds * 1_000_000)
        }

        let body: HTTPBodySequence
        if let data {
            body = .init(data: data)
        } else if let fileURL = fileURL {
            body = try .init(file: fileURL)
        } else {
            body = .init(data: Data())
        }

        let headers: [HTTPHeader: String]? = headers?.reduce(into: [:]) { result, pair in
            result[HTTPHeader(rawValue: pair.key)] = pair.value
        }
        let code = statusCode.flatMap { HTTPStatusCode($0, phrase: "Stubbed") } ?? .ok

        return HTTPResponse(statusCode: code, headers: headers ?? [:], body: body)
    }
}

// MARK: - Creating HTTP stubs

typealias RequestFilter = (URLRequest) -> Bool
typealias ResponseBlock = (URLRequest) -> HTTPStubsResponse

extension HTTPStub {

    func stub(condition: @escaping RequestFilter, response: @escaping ResponseBlock) {
        stubs.append((condition, response))
    }

}

func isPath(_ path: String) -> RequestFilter {
    { $0.url?.path == path }
}

func hasHeaderNamed(_ name: String, value: String?) -> RequestFilter {
    { $0.value(forHTTPHeaderField: name) == value }
}
