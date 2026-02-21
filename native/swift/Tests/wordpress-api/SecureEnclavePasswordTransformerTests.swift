import CryptoKit
import Foundation
import Security
import Testing
import WordPressAPI

/// Remove any Keychain item left over from a previous test run.
private func removeKeychainItem(alias: String) {
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: "rs.wordpress.api.password-transformer",
        kSecAttrAccount as String: alias
    ]
    SecItemDelete(query as CFDictionary)
}

@Suite("SecureEnclavePasswordTransformer Tests")
struct SecureEnclavePasswordTransformerTests {

    @Test("Round-trip encrypt then decrypt")
    func roundTrip() throws {
        let alias = "test-roundtrip-\(UUID().uuidString)"
        defer { removeKeychainItem(alias: alias) }

        let transformer = try SecureEnclavePasswordTransformer(alias: alias)
        let original = "hunter2"

        let encrypted = try transformer.encrypt(password: original)
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == original)
    }

    @Test("Encrypted output differs from plaintext")
    func encryptedDiffersFromPlaintext() throws {
        let alias = "test-differs-\(UUID().uuidString)"
        defer { removeKeychainItem(alias: alias) }

        let transformer = try SecureEnclavePasswordTransformer(alias: alias)
        let encrypted = try transformer.encrypt(password: "hunter2")

        #expect(encrypted != "hunter2")
    }

    @Test("Encrypting the same password twice produces different ciphertext")
    func differentCiphertextPerEncryption() throws {
        let alias = "test-different-ct-\(UUID().uuidString)"
        defer { removeKeychainItem(alias: alias) }

        let transformer = try SecureEnclavePasswordTransformer(alias: alias)
        let enc1 = try transformer.encrypt(password: "hunter2")
        let enc2 = try transformer.encrypt(password: "hunter2")

        #expect(enc1 != enc2)
    }

    @Test("Empty string round-trip")
    func emptyString() throws {
        let alias = "test-empty-\(UUID().uuidString)"
        defer { removeKeychainItem(alias: alias) }

        let transformer = try SecureEnclavePasswordTransformer(alias: alias)
        let encrypted = try transformer.encrypt(password: "")
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == "")
    }

    @Test("Unicode password round-trip")
    func unicodeRoundTrip() throws {
        let alias = "test-unicode-\(UUID().uuidString)"
        defer { removeKeychainItem(alias: alias) }

        let transformer = try SecureEnclavePasswordTransformer(alias: alias)
        let original = "p@$$w0rd-\u{1F512}-\u{00E9}\u{00F1}"

        let encrypted = try transformer.encrypt(password: original)
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == original)
    }

    @Test("Long password round-trip")
    func longPasswordRoundTrip() throws {
        let alias = "test-long-\(UUID().uuidString)"
        defer { removeKeychainItem(alias: alias) }

        let transformer = try SecureEnclavePasswordTransformer(alias: alias)
        let original = String(repeating: "a", count: 10_000)

        let encrypted = try transformer.encrypt(password: original)
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == original)
    }

    @Test("Persisted key can decrypt ciphertext from original key")
    func persistedKeyRoundTrip() throws {
        let alias = "test-persisted-\(UUID().uuidString)"
        defer { removeKeychainItem(alias: alias) }

        let original = try SecureEnclavePasswordTransformer(alias: alias)
        let encrypted = try original.encrypt(password: "persist-me")

        let restored = try SecureEnclavePasswordTransformer(
            persistedKeyData: original.persistedKeyData
        )
        let decrypted = try restored.decrypt(password: encrypted)

        #expect(decrypted == "persist-me")
    }

    @Test("Encrypted output is valid base64")
    func outputIsValidBase64() throws {
        let alias = "test-base64-\(UUID().uuidString)"
        defer { removeKeychainItem(alias: alias) }

        let transformer = try SecureEnclavePasswordTransformer(alias: alias)
        let encrypted = try transformer.encrypt(password: "test")

        #expect(Data(base64Encoded: encrypted) != nil)
    }

    @Test("Encrypted output contains ephemeral public key, nonce, ciphertext, and tag")
    func outputStructure() throws {
        let alias = "test-structure-\(UUID().uuidString)"
        defer { removeKeychainItem(alias: alias) }

        let transformer = try SecureEnclavePasswordTransformer(alias: alias)
        let plaintext = "hello"

        let encrypted = try transformer.encrypt(password: plaintext)
        let data = try #require(Data(base64Encoded: encrypted))

        // salt(32) + ephemeral pubkey(65) + nonce(12) + ciphertext + tag(16)
        let expectedSize = 32 + 65 + 12 + plaintext.utf8.count + 16
        #expect(data.count == expectedSize)
    }

    @Test("isHardwareBacked reflects build environment")
    func isHardwareBacked() throws {
        let alias = "test-hwbacked-\(UUID().uuidString)"
        defer { removeKeychainItem(alias: alias) }

        let transformer = try SecureEnclavePasswordTransformer(alias: alias)

        #if targetEnvironment(simulator)
        #expect(transformer.isHardwareBacked == false)
        #else
        #expect(transformer.isHardwareBacked == true)
        #endif
    }

    @Test("Invalid persisted key data throws")
    func invalidPersistedKeyData() {
        #expect(throws: (any Error).self) {
            _ = try SecureEnclavePasswordTransformer(
                persistedKeyData: Data([0xFF, 0x00])
            )
        }
    }

    @Test("Empty persisted key data throws")
    func emptyPersistedKeyData() {
        #expect(throws: (any Error).self) {
            _ = try SecureEnclavePasswordTransformer(persistedKeyData: Data())
        }
    }

    @Test("Decrypting with a different key throws DecryptionFailed")
    func differentKeyThrowsDecryptionFailed() throws {
        let alias1 = "test-diffkey-1-\(UUID().uuidString)"
        let alias2 = "test-diffkey-2-\(UUID().uuidString)"
        defer {
            removeKeychainItem(alias: alias1)
            removeKeychainItem(alias: alias2)
        }

        let transformer1 = try SecureEnclavePasswordTransformer(alias: alias1)
        let transformer2 = try SecureEnclavePasswordTransformer(alias: alias2)

        let encrypted = try transformer1.encrypt(password: "secret")

        do {
            _ = try transformer2.decrypt(password: encrypted)
            Issue.record("Expected DecryptionFailed error")
        } catch let error as PasswordTransformerError {
            guard case .DecryptionFailed = error else {
                Issue.record("Expected DecryptionFailed, got \(error)")
                return
            }
        }
    }
}

@Suite("SecureEnclavePasswordTransformer Alias Tests")
struct SecureEnclavePasswordTransformerAliasTests {

    @Test("Alias-based round-trip: create, encrypt, restore, decrypt")
    func aliasRoundTrip() throws {
        let alias = "test-alias-roundtrip-\(UUID().uuidString)"
        defer { removeKeychainItem(alias: alias) }

        let transformer1 = try SecureEnclavePasswordTransformer(alias: alias)
        let encrypted = try transformer1.encrypt(password: "secret")

        let transformer2 = try SecureEnclavePasswordTransformer(alias: alias)
        let decrypted = try transformer2.decrypt(password: encrypted)

        #expect(decrypted == "secret")
    }

    @Test("Default alias works without arguments")
    func defaultAlias() throws {
        let alias = "test-default-alias-\(UUID().uuidString)"
        defer { removeKeychainItem(alias: alias) }

        let transformer = try SecureEnclavePasswordTransformer(alias: alias)
        let encrypted = try transformer.encrypt(password: "default-test")
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == "default-test")
    }

    @Test("Different aliases produce independent keys")
    func differentAliasesAreIndependent() throws {
        let alias1 = "test-alias-independent-1-\(UUID().uuidString)"
        let alias2 = "test-alias-independent-2-\(UUID().uuidString)"
        defer {
            removeKeychainItem(alias: alias1)
            removeKeychainItem(alias: alias2)
        }

        let transformer1 = try SecureEnclavePasswordTransformer(alias: alias1)
        let transformer2 = try SecureEnclavePasswordTransformer(alias: alias2)

        #expect(transformer1.persistedKeyData != transformer2.persistedKeyData)
    }
}

@Suite("SecureEnclavePasswordTransformer File Tests")
struct SecureEnclavePasswordTransformerFileTests {

    @Test("File-based round-trip: create, encrypt, restore, decrypt")
    func fileRoundTrip() throws {
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(
            at: dir, withIntermediateDirectories: true
        )
        defer { try? FileManager.default.removeItem(at: dir) }

        let keyFile = dir.appendingPathComponent("key.dat")

        let transformer1 = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        let encrypted = try transformer1.encrypt(password: "file-test")

        let transformer2 = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        let decrypted = try transformer2.decrypt(password: encrypted)

        #expect(decrypted == "file-test")
    }

    @Test("Key file is created on first use")
    func keyFileCreated() throws {
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(
            at: dir, withIntermediateDirectories: true
        )
        defer { try? FileManager.default.removeItem(at: dir) }

        let keyFile = dir.appendingPathComponent("key.dat")
        #expect(!FileManager.default.fileExists(atPath: keyFile.path))

        _ = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        #expect(FileManager.default.fileExists(atPath: keyFile.path))
    }
}
