import Foundation
import WordPressAPI

extension WpNetworkHeaderMap {
    static var empty: WpNetworkHeaderMap {
        // swiftlint:disable:next force_try
        try! WpNetworkHeaderMap.fromMap(hashMap: [:])
    }

    static func withLinkHeader(_ value: String) -> WpNetworkHeaderMap {
        // swiftlint:disable:next force_try
        try! WpNetworkHeaderMap.fromMap(hashMap: ["Link": value])
    }
}

extension PaginatableResponse {
    static var empty: Self {
        Self(data: [], headerMap: .empty, nextPageParams: nil, prevPageParams: nil)
    }
}

// These `Sendable` conformances are **NOT** safe – they're for the test suite only.
//
// Until or unless `WpNetworkRequest` and `WpNetworkRequest` become `uniffi::Record` (thus Structs)
// we can't guarantee that they're thread-safe

extension WpNetworkRequest: @unchecked Sendable {}
extension WpNetworkResponse: @unchecked Sendable {}

// This is only for testing – it's not production-ready
// extension WordPressLoginClientError: Equatable {
//    public static func == (lhs: WordPressLoginClientError, rhs: WordPressLoginClientError) -> Bool {
//        lhs.localizedDescription == rhs.localizedDescription
//    }
// }

func isLinux() -> Bool {
    #if os(Linux)
    return true
    #else
    return false
    #endif
}

extension WpNetworkResponse {
    static func retryAfter(_ seconds: Double) -> WpNetworkResponse {
        try! WpNetworkResponse(
            body: Data(),
            statusCode: 429,
            headerMap: .fromMap(hashMap: [
                "Retry-After": String(seconds)
            ])
        )
    }
}

extension HTTPURLResponse {
    static func from(_ response: WpNetworkResponse, request: WpNetworkRequest) -> HTTPURLResponse {

        guard let url = URL(string: request.url()) else {
            preconditionFailure("Invalid URL")
        }

        guard let httpResponse = HTTPURLResponse(
            url: url,
            statusCode: Int(response.statusCode),
            httpVersion: nil,
            headerFields: response.headerMap.toFlatMap()
        ) else {
            preconditionFailure("Invalid HTTPURLResponse")
        }

        return httpResponse
    }
}
