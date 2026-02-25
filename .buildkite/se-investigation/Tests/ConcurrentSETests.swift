import CryptoKit
import Foundation
import Testing

/// Experiment 1b: Do 15 concurrent SE tests exhaust the cooperative thread pool?
///
/// Swift Testing runs all tests concurrently by default. If each
/// SecureEnclave.P256.KeyAgreement.PrivateKey() blocks the cooperative thread,
/// the entire pool gets exhausted and ALL tests deadlock.
///
/// If this hangs but SingleSETest passes, the issue is cooperative pool exhaustion.
@Suite("Concurrent SE Tests")
struct ConcurrentSETests {
    @Test("SE key 01") func key01() throws {
        print("[EXP-1b] \(Date()) key01 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key01 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 02") func key02() throws {
        print("[EXP-1b] \(Date()) key02 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key02 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 03") func key03() throws {
        print("[EXP-1b] \(Date()) key03 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key03 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 04") func key04() throws {
        print("[EXP-1b] \(Date()) key04 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key04 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 05") func key05() throws {
        print("[EXP-1b] \(Date()) key05 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key05 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 06") func key06() throws {
        print("[EXP-1b] \(Date()) key06 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key06 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 07") func key07() throws {
        print("[EXP-1b] \(Date()) key07 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key07 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 08") func key08() throws {
        print("[EXP-1b] \(Date()) key08 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key08 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 09") func key09() throws {
        print("[EXP-1b] \(Date()) key09 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key09 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 10") func key10() throws {
        print("[EXP-1b] \(Date()) key10 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key10 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 11") func key11() throws {
        print("[EXP-1b] \(Date()) key11 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key11 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 12") func key12() throws {
        print("[EXP-1b] \(Date()) key12 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key12 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 13") func key13() throws {
        print("[EXP-1b] \(Date()) key13 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key13 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 14") func key14() throws {
        print("[EXP-1b] \(Date()) key14 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key14 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }

    @Test("SE key 15") func key15() throws {
        print("[EXP-1b] \(Date()) key15 before SE key creation")
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("[EXP-1b] \(Date()) key15 after  SE key creation")
        #expect(key.publicKey.x963Representation.count == 65)
    }
}
