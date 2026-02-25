import CryptoKit
import Foundation
import Testing

/// Experiment 2: Does .serialized prevent the hang?
///
/// If ConcurrentSETests hangs but this passes, the root cause is cooperative
/// thread pool exhaustion from concurrent SE key creation. The fix would be
/// to add .serialized to the real SE test suites.
@Suite("Serialized SE Tests", .serialized)
struct SerializedSETests {
    @Test("Serialized SE key 01") func key01() throws {
        print("[EXP-2] \(Date()) key01 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key01 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 02") func key02() throws {
        print("[EXP-2] \(Date()) key02 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key02 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 03") func key03() throws {
        print("[EXP-2] \(Date()) key03 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key03 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 04") func key04() throws {
        print("[EXP-2] \(Date()) key04 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key04 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 05") func key05() throws {
        print("[EXP-2] \(Date()) key05 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key05 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 06") func key06() throws {
        print("[EXP-2] \(Date()) key06 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key06 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 07") func key07() throws {
        print("[EXP-2] \(Date()) key07 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key07 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 08") func key08() throws {
        print("[EXP-2] \(Date()) key08 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key08 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 09") func key09() throws {
        print("[EXP-2] \(Date()) key09 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key09 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 10") func key10() throws {
        print("[EXP-2] \(Date()) key10 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key10 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 11") func key11() throws {
        print("[EXP-2] \(Date()) key11 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key11 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 12") func key12() throws {
        print("[EXP-2] \(Date()) key12 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key12 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 13") func key13() throws {
        print("[EXP-2] \(Date()) key13 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key13 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 14") func key14() throws {
        print("[EXP-2] \(Date()) key14 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key14 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("Serialized SE key 15") func key15() throws {
        print("[EXP-2] \(Date()) key15 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-2] \(Date()) key15 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }
}
