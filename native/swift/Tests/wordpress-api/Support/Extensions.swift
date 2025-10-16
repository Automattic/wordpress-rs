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

let isXCTest: Bool = Bundle.main.infoDictionary?["CFBundleName"] as? String == "xctest"
