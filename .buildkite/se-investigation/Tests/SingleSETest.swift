import CryptoKit
import Foundation
import Testing

/// Experiment 1a: Does a SINGLE SecureEnclave key creation hang inside `swift test`?
///
/// If this hangs, the issue is that `swift test` binaries cannot access the
/// Secure Enclave at all (code signing / entitlements issue).
/// If this passes, concurrency is the trigger.
@Suite("Single SE Test")
struct SingleSETest {
    @Test("Create one SecureEnclave key")
    func createOneKey() throws {
        print("[EXP-1a] \(Date()) SecureEnclave.isAvailable = \(SecureEnclave.isAvailable)")
        print("[EXP-1a] \(Date()) Before SecureEnclave.P256.KeyAgreement.PrivateKey()")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1a] \(Date()) After  SecureEnclave.P256.KeyAgreement.PrivateKey()")
        #expect(key.publicKey.x963Representation.count == 65)
        print("[EXP-1a] \(Date()) Test complete")
    }
}
