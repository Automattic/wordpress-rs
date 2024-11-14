import Foundation
import WordPressAPI

final class HTTPStubs: SafeRequestExecutor {

    typealias Stub = (condition: @Sendable (WpNetworkRequest) -> Bool, response: WpNetworkResponse)

    private let stubs: [Stub]
    private let missingStub: Result<WpNetworkResponse, Error>?

    init(stubs: [Stub] = [], missingStub: Result<WpNetworkResponse, Error>? = nil) {
        self.stubs = stubs
        self.missingStub = missingStub
    }

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

    private func stub(for request: WpNetworkRequest) -> WpNetworkResponse? {
        stubs.first { stub in stub.condition(request) }?
            .response
    }

    static func stub(url: String, with response: WpNetworkResponse) -> Stub {
        (
            condition: { URL(string: $0.url()) == URL(string: url) },
            response: response
        )
    }

    static func stub(host: String, with response: WpNetworkResponse) -> Stub {
        (
            condition: { URL(string: $0.url())?.host == host },
            response: response
        )
    }

    static func stub(path: String, with response: WpNetworkResponse) -> Stub {
        (
            condition: { URL(string: $0.url())?.path == path },
            response: response
        )
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
