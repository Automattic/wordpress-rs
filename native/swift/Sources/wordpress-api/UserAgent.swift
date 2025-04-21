import Foundation

public struct UserAgent{
    public static var postfix: String {
        #if canImport(Darwin)
        "\(bundleName)/\(bundleVersion) \(cfNetworkVersion) \(darwinVersion) \(architecture)"
        #elseif os(Linux)
        "Linux/URLSession \(architecture)"
        #else
        "unknown"
        #endif
    }

    #if canImport(Darwin)
    static var bundleName: String {
        Bundle.main.infoDictionary?["CFBundleName"] as? String ?? "application"
    }

    static var bundleVersion: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "1.0"
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
        let bytes = Data(bytes: &systemInfo.release, count: Int(_SYS_NAMELEN))
        guard let string = String(bytes: bytes, encoding: .ascii) else {
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
        var value = [CChar](repeating: 0,  count: size)
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
        } catch let err {
            return "unknown"
        }
    }
    #endif
}
