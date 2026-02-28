import Foundation
import WordPressAPI

#if canImport(Security)
import Security
#endif

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

/// Returns true if the test CA certificate is trusted in the system keychain.
/// Run `make trust-test-ca` to trust it. On CI VMs where trust modification
/// isn't possible, this returns false and dependent tests will be skipped.
func isTestCATrusted() -> Bool {
    #if canImport(Security)
    guard let pemUrl = Bundle.module.url(forResource: "ca-cert", withExtension: "pem", subdirectory: "ssl-certs"),
          let pemData = try? Data(contentsOf: pemUrl),
          let pemString = String(data: pemData, encoding: .utf8) else {
        return false
    }

    let base64 = pemString
        .components(separatedBy: "\n")
        .filter { !$0.hasPrefix("-----") && !$0.isEmpty }
        .joined()
    guard let derData = Data(base64Encoded: base64),
          let cert = SecCertificateCreateWithDER(nil, derData as CFData) else {
        return false
    }

    var trust: SecTrust?
    let policy = SecPolicyCreateBasicX509()
    guard SecTrustCreateWithCertificates(cert as CFTypeRef, policy, &trust) == errSecSuccess,
          let trust else {
        return false
    }

    return SecTrustEvaluateWithError(trust, nil)
    #else
    return false
    #endif
}

let isXCTest: Bool = Bundle.main.infoDictionary?["CFBundleName"] as? String == "xctest"
