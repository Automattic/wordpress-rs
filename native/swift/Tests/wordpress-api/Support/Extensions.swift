import Foundation
import XCTest

@testable import WordPressAPIInternal

extension WpNetworkHeaderMap {
    static var empty: WpNetworkHeaderMap {
        try! WpNetworkHeaderMap.fromMap(hashMap: [:])
    }

    static func withLinkHeader(_ value: String) -> WpNetworkHeaderMap {
        try! WpNetworkHeaderMap.fromMap(hashMap: ["Link": value])
    }
}
