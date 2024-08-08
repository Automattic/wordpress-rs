import Foundation
import WordPressAPI

#if canImport(WordPressAPIInternal)
import WordPressAPIInternal
#endif

class HTTPStubs: SafeRequestExecutor {

    var stubs: [(condition: (WpNetworkRequest) -> Bool, response: WpNetworkResponse)] = []

    var missingStub: Result<WpNetworkResponse, Error>?

    public func execute(_ request: WpNetworkRequest) async -> Result<WpNetworkResponse, RequestExecutionError> {
        if let response = stub(for: request) {
            return .success(response)
        }

        switch missingStub {
        case let .success(response):
            return .success(response)
        case .failure:
            // TODO: Translate error into the Rust type
            return .failure(.RequestExecutionFailed(statusCode: nil, reason: ""))
        default:
            // TODO: Translate error into the Rust type
            return .failure(.RequestExecutionFailed(statusCode: nil, reason: ""))
        }
    }

    func stub(for request: WpNetworkRequest) -> WpNetworkResponse? {
        stubs.first { stub in stub.condition(request) }?
            .response
    }

    func stub(url: String, with response: WpNetworkResponse) {
        stubs.append((
            condition: { URL(string: $0.url()) == URL(string: url) },
            response: response
        ))
    }

    func stub(host: String, with response: WpNetworkResponse) {
        stubs.append((
            condition: { URL(string: $0.url())?.host == host },
            response: response
        ))
    }

    func stub(path: String, with response: WpNetworkResponse) {
        stubs.append((
            condition: { URL(string: $0.url())?.path == path },
            response: response
        ))
    }

}

extension WpNetworkResponse {

    static func json(_ content: String) throws -> WpNetworkResponse {
        WpNetworkResponse(
            body: content.data(using: .utf8)!,
            statusCode: 200,
            headerMap: try WpNetworkHeaderMap.fromMap(hashMap: ["Content-Type": "application/json"])
        )
    }

}
