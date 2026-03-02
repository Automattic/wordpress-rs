#if canImport(CryptoKit)
import CryptoKit
import Foundation
import Testing
@testable import WordPressAPI

/// Create a temporary key file URL for testing.
private func temporaryKeyFile() throws -> (URL, URL) {
    let dir = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(
        at: dir, withIntermediateDirectories: true
    )
    let keyFile = dir.appendingPathComponent("key.dat")
    return (dir, keyFile)
}

/// An in-memory ``KeychainStorage`` for testing without touching the real Keychain.
private final class InMemoryKeychainStorage: KeychainStorage {
    private var data: Data?

    func load() throws -> Data? {
        data
    }

    func save(data: Data) throws {
        self.data = data
    }
}

// MARK: - Serialized wrapper suite
//
// All Secure Enclave tests MUST run serially. Swift Testing executes tests
// concurrently on a fixed-size cooperative thread pool (~CPU core count).
// `SecureEnclave.P256.KeyAgreement.PrivateKey()` blocks the calling thread
// while the Secure Enclave processes the request. When many tests create SE
// keys concurrently, every cooperative thread gets blocked and the entire
// test runner deadlocks — including unrelated tests waiting for a free thread.
//
// `.serialized` only applies within a single @Suite, so we nest all SE test
// suites inside this parent suite to ensure they never run concurrently with
// each other.
//
// See: https://forums.swift.org/t/cooperative-pool-deadlock-when-calling-into-an-opaque-subsystem/70685

@Suite("SecureEnclavePasswordTransformer", .serialized)
struct SecureEnclavePasswordTransformerAllTests {

    @Test("Round-trip encrypt then decrypt")
    func roundTrip() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        let original = "hunter2"

        let encrypted = try transformer.encrypt(password: original)
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == original)
    }

    @Test("Encrypted output differs from plaintext")
    func encryptedDiffersFromPlaintext() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        let encrypted = try transformer.encrypt(password: "hunter2")

        #expect(encrypted != "hunter2")
    }

    @Test("Encrypting the same password twice produces different ciphertext")
    func differentCiphertextPerEncryption() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        let enc1 = try transformer.encrypt(password: "hunter2")
        let enc2 = try transformer.encrypt(password: "hunter2")

        #expect(enc1 != enc2)
    }

    @Test("Empty string round-trip")
    func emptyString() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        let encrypted = try transformer.encrypt(password: "")
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == "")
    }

    @Test("Unicode password round-trip")
    func unicodeRoundTrip() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        let original = "p@$$w0rd-\u{1F512}-\u{00E9}\u{00F1}"

        let encrypted = try transformer.encrypt(password: original)
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == original)
    }

    @Test("Long password round-trip")
    func longPasswordRoundTrip() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        let original = String(repeating: "a", count: 10_000)

        let encrypted = try transformer.encrypt(password: original)
        let decrypted = try transformer.decrypt(password: encrypted)

        #expect(decrypted == original)
    }

    @Test("Persisted key can decrypt ciphertext from original key")
    func persistedKeyRoundTrip() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let original = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        let encrypted = try original.encrypt(password: "persist-me")

        let restored = try SecureEnclavePasswordTransformer(
            persistedKeyData: original.persistedKeyData
        )
        let decrypted = try restored.decrypt(password: encrypted)

        #expect(decrypted == "persist-me")
    }

    @Test("Encrypted output is valid base64")
    func outputIsValidBase64() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        let encrypted = try transformer.encrypt(password: "test")

        #expect(Data(base64Encoded: encrypted) != nil)
    }

    @Test("Encrypted output contains ephemeral public key, nonce, ciphertext, and tag")
    func outputStructure() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        let plaintext = "hello"

        let encrypted = try transformer.encrypt(password: plaintext)
        let data = try #require(Data(base64Encoded: encrypted))

        // salt(32) + ephemeral pubkey(65) + nonce(12) + ciphertext + tag(16)
        let expectedSize = 32 + 65 + 12 + plaintext.utf8.count + 16
        #expect(data.count == expectedSize)
    }

    @Test("isHardwareBacked reflects build environment")
    func isHardwareBacked() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)

        #if targetEnvironment(simulator)
        #expect(transformer.isHardwareBacked == false)
        #else
        #expect(transformer.isHardwareBacked == SecureEnclave.isAvailable)
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
        let (dir1, keyFile1) = try temporaryKeyFile()
        let (dir2, keyFile2) = try temporaryKeyFile()
        defer {
            try? FileManager.default.removeItem(at: dir1)
            try? FileManager.default.removeItem(at: dir2)
        }

        let transformer1 = try SecureEnclavePasswordTransformer(keyFile: keyFile1)
        let transformer2 = try SecureEnclavePasswordTransformer(keyFile: keyFile2)

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

    @Test("Decrypting invalid base64 throws DecryptionFailed with reason")
    func decryptInvalidBase64() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)

        do {
            _ = try transformer.decrypt(password: "not valid base64!!!")
            Issue.record("Expected DecryptionFailed error")
        } catch let error as PasswordTransformerError {
            guard case .DecryptionFailed(let reason) = error else {
                Issue.record("Expected DecryptionFailed, got \(error)")
                return
            }
            #expect(reason.contains("base64"))
        }
    }

    @Test("Decrypting truncated ciphertext throws DecryptionFailed with reason")
    func decryptTruncatedCiphertext() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)

        // Valid base64, but far too short to contain salt + pubkey + nonce + tag
        let tooShort = Data(repeating: 0xAA, count: 10).base64EncodedString()

        do {
            _ = try transformer.decrypt(password: tooShort)
            Issue.record("Expected DecryptionFailed error")
        } catch let error as PasswordTransformerError {
            guard case .DecryptionFailed(let reason) = error else {
                Issue.record("Expected DecryptionFailed, got \(error)")
                return
            }
            #expect(reason.contains("too short"))
        }
    }

    @Test("Decrypting ciphertext with invalid public key throws DecryptionFailed")
    func decryptInvalidPublicKey() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)

        // Right total size (salt=32 + pubkey=65 + nonce=12 + tag=16 = 125) but garbage pubkey bytes
        let garbage = Data(repeating: 0xFF, count: 125)

        do {
            _ = try transformer.decrypt(password: garbage.base64EncodedString())
            Issue.record("Expected DecryptionFailed error")
        } catch let error as PasswordTransformerError {
            guard case .DecryptionFailed(let reason) = error else {
                Issue.record("Expected DecryptionFailed, got \(error)")
                return
            }
            #expect(reason.contains("public key") || reason.contains("sealed box"))
        }
    }

    @Test("Decrypting tampered ciphertext throws DecryptionFailed")
    func decryptTamperedCiphertext() throws {
        let (dir, keyFile) = try temporaryKeyFile()
        defer { try? FileManager.default.removeItem(at: dir) }

        let transformer = try SecureEnclavePasswordTransformer(keyFile: keyFile)
        let encrypted = try transformer.encrypt(password: "tamper-test")

        var data = try #require(Data(base64Encoded: encrypted))
        // Flip the last byte (inside the GCM tag)
        data[data.count - 1] ^= 0xFF

        do {
            _ = try transformer.decrypt(password: data.base64EncodedString())
            Issue.record("Expected DecryptionFailed error")
        } catch let error as PasswordTransformerError {
            guard case .DecryptionFailed(let reason) = error else {
                Issue.record("Expected DecryptionFailed, got \(error)")
                return
            }
            #expect(reason.contains("authentication failed") || reason.contains("different key"))
        }
    }

    @Suite("Storage Tests")
    struct StorageTests {

        @Test("Storage round-trip: create, encrypt, restore, decrypt")
        func storageRoundTrip() throws {
            let storage = InMemoryKeychainStorage()

            let transformer1 = try SecureEnclavePasswordTransformer(
                applicationName: "test-roundtrip", keychainStorage: storage
            )
            let encrypted = try transformer1.encrypt(password: "secret")

            let transformer2 = try SecureEnclavePasswordTransformer(
                applicationName: "test-roundtrip", keychainStorage: storage
            )
            let decrypted = try transformer2.decrypt(password: encrypted)

            #expect(decrypted == "secret")
        }

        @Test("Application name works with storage")
        func applicationNameWithStorage() throws {
            let storage = InMemoryKeychainStorage()

            let transformer = try SecureEnclavePasswordTransformer(
                applicationName: "test-default-name", keychainStorage: storage
            )
            let encrypted = try transformer.encrypt(password: "default-test")
            let decrypted = try transformer.decrypt(password: encrypted)

            #expect(decrypted == "default-test")
        }

        @Test("Different application names produce independent keys")
        func differentNamesAreIndependent() throws {
            let storage1 = InMemoryKeychainStorage()
            let storage2 = InMemoryKeychainStorage()

            let transformer1 = try SecureEnclavePasswordTransformer(
                applicationName: "name-1", keychainStorage: storage1
            )
            let transformer2 = try SecureEnclavePasswordTransformer(
                applicationName: "name-2", keychainStorage: storage2
            )

            #expect(transformer1.persistedKeyData != transformer2.persistedKeyData)
        }
    }

    @Suite("File Tests")
    struct FileTests {

        @Test("File-based round-trip: create, encrypt, restore, decrypt")
        func fileRoundTrip() throws {
            let (dir, keyFile) = try temporaryKeyFile()
            defer { try? FileManager.default.removeItem(at: dir) }

            let transformer1 = try SecureEnclavePasswordTransformer(keyFile: keyFile)
            let encrypted = try transformer1.encrypt(password: "file-test")

            let transformer2 = try SecureEnclavePasswordTransformer(keyFile: keyFile)
            let decrypted = try transformer2.decrypt(password: encrypted)

            #expect(decrypted == "file-test")
        }

        @Test("Key file is created on first use")
        func keyFileCreated() throws {
            let (dir, keyFile) = try temporaryKeyFile()
            defer { try? FileManager.default.removeItem(at: dir) }

            #expect(!FileManager.default.fileExists(atPath: keyFile.path))

            _ = try SecureEnclavePasswordTransformer(keyFile: keyFile)
            #expect(FileManager.default.fileExists(atPath: keyFile.path))
        }
    }
}

#endif
