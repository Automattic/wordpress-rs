#if COCOAPODS

import Foundation

private class BundleFinder {}

extension Bundle {
    static var module: Bundle {
        let url = Bundle(for: BundleFinder.self).url(forResource: "Resources", withExtension: "bundle")!
        try? Bundle(for: BundleFinder.self).bundleURL.absoluteString.write(toFile: "/Users/tonyli/Projects/wordpress-rs/file.path", atomically: true, encoding: .utf8)
        return Bundle(url: url)!
    }
}

#endif
