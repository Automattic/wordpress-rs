import Foundation

public struct UserAgent {
    public static var postfix: String {
        #if canImport(Darwin)
        if let bundleName = bundleName, let bundleVersion = bundleVersion {
            return "\(bundleName)/\(bundleVersion) \(cfNetworkVersion) \(darwinVersion) \(architecture)"
        } else {
            return "\(cfNetworkVersion) \(darwinVersion) \(architecture)"
        }
        #elseif os(Linux)
        "Linux/URLSession \(architecture)"
        #else
        "unknown"
        #endif
    }

    #if canImport(Darwin)
    static var bundleName: String? {
        Bundle.main.infoDictionary?["CFBundleName"] as? String
    }

    static var bundleVersion: String? {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String
    }

    static var cfNetworkVersion: String {
        guard
            let bundle = Bundle(identifier: "com.apple.CFNetwork"),
            let versionAny = bundle.infoDictionary?[kCFBundleVersionKey as String],
            let version = versionAny as? String
        else {
            return "CFNetwork/unknown"
        }

        return "CFNetwork/\(version)"
    }

    static var darwinVersion: String {
        var systemInfo = utsname()
        uname(&systemInfo)

        // Picking a sensible buffer size manually means the value might be truncated. 8 bytes is enough for a very long
        // version string – right now we only need 6 bytes for a three-segment code (ex: `24.3.0` ~> 6 bytes). We can
        // use something like `_SYS_NAMELEN` as a buffer size but that's 256 bytes and it's a big waste of CPU
        // to `.filter` over.
        let bytes = Data(bytes: &systemInfo.release, count: 8).filter { $0 != 0 }

        guard let string = String(data: bytes, encoding: .ascii) else {
            return "Darwin/unknown"
        }

        return "Darwin/\(string)"
    }

    static var architecture: String {
        systemInfo(for: "hw.machine") ?? "unknown"
    }

    static func systemInfo(for key: String) -> String? {
        var size = 0
        sysctlbyname(key, nil, &size, nil, 0)
        var value = [CChar](repeating: 0, count: size)
        sysctlbyname(key, &value, &size, nil, 0)
        guard let string = String(cString: value, encoding: .ascii) else {
            return nil
        }

        return string
    }
    #elseif os(Linux)
    static var architecture: String {
        do {
            let process = Process()
            let pipe = Pipe()

            process.standardOutput = pipe // you can also set stderr and stdin
            process.executableURL = URL(fileURLWithPath: "/usr/bin/uname") // or any other shell you like
            process.arguments = ["-m"]

            try process.run()

            let data = pipe.fileHandleForReading.readDataToEndOfFile()

            guard let string = String(data: data, encoding: .ascii) else {
                return "unknown"
            }

            return string.trimmingCharacters(in: .whitespacesAndNewlines)
        } catch {
            return "unknown"
        }
    }
    #endif
}
