import CryptoKit
import Foundation
import Testing

/// Experiment 3: Can Swift Testing's .timeLimit interrupt a blocked SE call?
///
/// If the timeout fires, we can use .timeLimit as a safety net.
/// If it hangs past 1 minute, .timeLimit uses cooperative cancellation
/// which can't interrupt a truly blocked thread.
@Suite("Timeout SE Test")
struct TimeoutSETest {
    @Test("SE key with 1-minute timeout", .timeLimit(.minutes(1)))
    func keyWithTimeout() throws {
        print("[EXP-3] \(Date()) SecureEnclave.isAvailable = \(SecureEnclave.isAvailable)")
        print("[EXP-3] \(Date()) Before SecureEnclave.P256.KeyAgreement.PrivateKey()")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-3] \(Date()) After  SecureEnclave.P256.KeyAgreement.PrivateKey()")
        #expect(key.publicKey.x963Representation.count == 65)
        print("[EXP-3] \(Date()) Test complete")
    }
}
