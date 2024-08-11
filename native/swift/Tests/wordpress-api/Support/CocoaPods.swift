#if COCOAPODS

import Foundation

private class BundleFinder {}

extension Bundle {
    static var module: Bundle {
        let url = Bundle(for: BundleFinder.self).url(forResource: "Resources", withExtension: "bundle")!
        return Bundle(url: url)!
    }
}

#endif
