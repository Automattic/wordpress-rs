#if os(Linux)
import Foundation
import Testing
import WordPressAPI

/// Tests for the Rust-backed AesGcmPasswordTransformer exposed via UniFFI.
///
/// AesGcmPasswordTransformer is only available on Linux where the Rust library
/// is built with default features (including aes-gcm-encryption). On Apple
/// platforms the xcframework is built with --no-default-features, so the type
/// doesn't exist there — Apple platforms use SecureEnclavePasswordTransformer
/// instead.
@Suite("AesGcmPasswordTransformer")
struct AesGcmPasswordTransformerTests {

    @Test("Round-trip encrypt then decrypt")
    func roundTrip() throws {
        let transformer = AesGcmPasswordTransformer(sharedSecret: "test-secret")
        let original = "hunter2"

        let encrypted = try transformer.encrypt(password: original)
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == original)
    }

    @Test("Encrypted output differs from plaintext")
    func encryptedDiffersFromPlaintext() throws {
        let transformer = AesGcmPasswordTransformer(sharedSecret: "test-secret")
        let encrypted = try transformer.encrypt(password: "hunter2")

        #expect(encrypted != "hunter2")
    }

    @Test("Encrypting the same password twice produces different ciphertext")
    func differentCiphertextPerEncryption() throws {
        let transformer = AesGcmPasswordTransformer(sharedSecret: "test-secret")
        let enc1 = try transformer.encrypt(password: "hunter2")
        let enc2 = try transformer.encrypt(password: "hunter2")

        #expect(enc1 != enc2)
    }

    @Test("Wrong key fails to decrypt")
    func wrongKeyFails() throws {
        let t1 = AesGcmPasswordTransformer(sharedSecret: "key-one")
        let t2 = AesGcmPasswordTransformer(sharedSecret: "key-two")

        let encrypted = try t1.encrypt(password: "secret")

        #expect(throws: (any Error).self) {
            _ = try t2.decrypt(password: encrypted)
        }
    }

    @Test("Empty string round-trip")
    func emptyString() throws {
        let transformer = AesGcmPasswordTransformer(sharedSecret: "test-secret")
        let encrypted = try transformer.encrypt(password: "")
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == "")
    }

    @Test("Unicode password round-trip")
    func unicodeRoundTrip() throws {
        let transformer = AesGcmPasswordTransformer(sharedSecret: "test-secret")
        let original = "p@$$w0rd-\u{1F512}-\u{00E9}\u{00F1}"

        let encrypted = try transformer.encrypt(password: original)
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == original)
    }

    @Test("Long password round-trip")
    func longPasswordRoundTrip() throws {
        let transformer = AesGcmPasswordTransformer(sharedSecret: "test-secret")
        let original = String(repeating: "a", count: 10_000)

        let encrypted = try transformer.encrypt(password: original)
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == original)
    }

    @Test("Conforms to PasswordTransformer protocol")
    func conformsToProtocol() throws {
        let transformer: any PasswordTransformer = AesGcmPasswordTransformer(
            sharedSecret: "test-secret"
        )

        let encrypted = try transformer.encrypt(password: "test")
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == "test")
    }
}
#endif
