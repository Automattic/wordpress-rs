import Foundation
import XCTest

@testable import WordPressAPIInternal

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

// These `Sendable` conformances are **NOT** safe – they're for the test suite only.
//
// Until or unless `WpNetworkRequest` and `WpNetworkRequest` become `uniffi::Record` (thus Structs)
// we can't guarantee that they're thread-safe

extension WpNetworkRequest: @unchecked Sendable {}
extension WpNetworkResponse: @unchecked Sendable {}

// This is only for testing – it's not production-ready
extension WordPressLoginClient.Error: Equatable {
    public static func == (lhs: WordPressLoginClient.Error, rhs: WordPressLoginClient.Error) -> Bool {
        lhs.localizedDescription == rhs.localizedDescription
    }
}
