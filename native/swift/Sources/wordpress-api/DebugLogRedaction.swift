import Foundation

/// Keeps credentials out of what the library prints about a request for debugging.
enum DebugLogRedaction {
    static let redacted = "<redacted>"

    /// Headers whose value is a credential, in lower case.
    static let credentialHeaders: Set<String> = [
        "authorization",
        "proxy-authorization",
        "cookie",
        "set-cookie",
        "x-wp-nonce"
    ]

    /// Headers whose value starts with an authentication scheme, such as `Bearer`, which is kept.
    private static let schemeHeaders: Set<String> = ["authorization", "proxy-authorization"]

    /// `value` as it can be printed: unchanged, unless `name` is one of `credentialHeaders`, matched without
    /// regard to case. Then it's `<redacted>`, after the scheme for an `Authorization` header.
    static func headerValue(_ value: String, forHeader name: String) -> String {
        let lowercaseName = name.lowercased()
        guard credentialHeaders.contains(lowercaseName) else {
            return value
        }

        let parts = value.trimmingCharacters(in: .whitespaces).split(separator: " ", maxSplits: 1)
        if schemeHeaders.contains(lowercaseName), parts.count == 2, isScheme(parts[0]) {
            return "\(parts[0]) \(redacted)"
        }
        return redacted
    }

    /// The lines `WpNetworkRequest.debugPrint()` prints, with credentials left out.
    static func requestLines(method: String, url: String, headers: [String: [String]], body: String?) -> [String] {
        var lines = ["\(method) \(url)"]
        for (name, values) in headers.sorted(by: { $0.key < $1.key }) {
            for value in values {
                lines.append("\(name): \(headerValue(value, forHeader: name))")
            }
        }
        lines.append("")
        if let body {
            lines.append("<body of \(body.utf8.count) bytes, not printed>")
        }
        return lines
    }

    private static func isScheme(_ text: Substring) -> Bool {
        text.allSatisfy { $0.isASCII && ($0.isLetter || $0.isNumber || "-_.".contains($0)) }
    }
}
