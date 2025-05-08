import Foundation
import WordPressAPI

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

final class HTTPStubs: SafeRequestExecutor {
    typealias Stub = (condition: @Sendable (WpNetworkRequest) -> Bool, response: WpNetworkResponse)

    private let stubs: [Stub]
    private let missingStub: Result<WpNetworkResponse, Error>?

    init(stubs: [Stub] = [], missingStub: Result<WpNetworkResponse, Error>? = nil) {
        self.stubs = stubs
        self.missingStub = missingStub
    }

    func withCredential(_ credential: URLCredential) -> Self {
        self
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
            return .failure(
                .RequestExecutionFailed(statusCode: nil, redirects: nil, reason: .genericError(errorMessage: ""))
            )
        default:
            // TODO: Translate error into the Rust type
            return .failure(
                .RequestExecutionFailed(statusCode: nil, redirects: nil, reason: .genericError(errorMessage: ""))
            )
        }
    }

    func uploadMedia(mediaUploadRequest: MediaUploadRequest) async -> Result<WpNetworkResponse, RequestExecutionError> {
        preconditionFailure("This method is not yet implemented")
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

    func sleep(millis: UInt64) async {
        // swiftlint:disable:next force_try
        try! await Task.sleep(nanoseconds: millis * 1000)
    }
}

extension WpNetworkResponse {

    static func json(_ content: String) throws -> WpNetworkResponse {
        WpNetworkResponse(
            body: content.data(using: .utf8)!,
            statusCode: 200,
            responseHeaderMap: try WpNetworkHeaderMap.fromMap(hashMap: ["Content-Type": "application/json"]),
            requestUrl: "https://example.com",
            requestHeaderMap: .empty
        )
    }

    static func jsonResponse(named name: String, statusCode: UInt16 = 200) throws -> WpNetworkResponse {

        guard let resourceUrl = Bundle
            .module
            .url(forResource: name, withExtension: "json", subdirectory: "integration-test-responses")
        else {
            preconditionFailure("Could not find \(name).json")
        }

        return WpNetworkResponse(
            body: try Data(contentsOf: resourceUrl),
            statusCode: statusCode,
            responseHeaderMap: try WpNetworkHeaderMap.fromMap(hashMap: ["Content-Type": "application/json"]),
            requestUrl: "https://example.com",
            requestHeaderMap: .empty
        )
    }

    static func retryResponse(after: TimeInterval) throws -> WpNetworkResponse {
        return WpNetworkResponse(
            body: Data(),
            statusCode: 429,
            responseHeaderMap: try WpNetworkHeaderMap.fromMap(hashMap: ["Retry-After": String(Int(after))]),
            requestUrl: "https://example.com",
            requestHeaderMap: .empty
        )
    }

    static func withApiRoot(_ url: String) throws -> WpNetworkResponse {
        return WpNetworkResponse(
            body: Data(),
            statusCode: 200,
            responseHeaderMap: try WpNetworkHeaderMap.fromMap(hashMap: [
                "Link": "<\(url)>; rel=\"https://api.w.org/\""
            ]),
            requestUrl: url,
            requestHeaderMap: .empty
        )
    }
}
