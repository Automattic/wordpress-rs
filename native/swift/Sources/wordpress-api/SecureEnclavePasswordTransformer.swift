import CryptoKit
import Foundation
import WordPressAPIInternal

/// A `PasswordTransformer` implementation that uses ECIES (Elliptic Curve
/// Integrated Encryption Scheme) with AES-256-GCM.
///
/// On physical devices the P-256 private key is stored in the Secure Enclave
/// and never leaves the hardware. In Simulator builds a software P-256 key is
/// used instead so that development and testing work without hardware support.
/// Check ``isHardwareBacked`` to determine which mode is active.
///
/// ## How it works
///
/// - Each encryption generates an ephemeral P-256 key pair and a random
///   32-byte HKDF salt
/// - ECDH key agreement + HKDF-SHA256 derives a per-message AES-256-GCM key
/// - The salt and ephemeral public key are included in the output so
///   decryption can reconstruct the same symmetric key
///
/// ## Key Persistence
///
/// Use ``init(alias:)`` to auto-persist the key to the Keychain:
///
/// ```swift
/// // The key is created on first use and restored automatically on
/// // subsequent launches — no manual persistence needed.
/// let transformer = try SecureEnclavePasswordTransformer(alias: "my-app-key")
/// ```
///
/// Alternatively, you can restore a previously saved key via
/// ``init(persistedKeyData:)``. On physical devices ``persistedKeyData``
/// is an opaque Secure Enclave reference — it does **not** contain the
/// private key itself, so it can safely be stored in UserDefaults, a
/// file, or any other persistent store.
///
/// ## Encrypting and Decrypting
///
/// ```swift
/// let transformer = try SecureEnclavePasswordTransformer(alias: "my-key")
///
/// let encrypted = try transformer.encrypt(password: "hunter2")
/// let decrypted = try transformer.decrypt(password: encrypted)
/// // decrypted == "hunter2"
/// ```
public final class SecureEnclavePasswordTransformer: PasswordTransformer {

    private let privateKey: EciesPrivateKey

    /// The size of an uncompressed P-256 public key in X9.63 format.
    private static let publicKeySize = 65

    /// The size of the random HKDF salt generated per encryption.
    private static let saltSize = 32

    private static let sharedInfo = Data("wordpress-rs-password-encryption".utf8)

    private static let seKeyTag: UInt8 = 0x01
    private static let softwareKeyTag: UInt8 = 0x02

    /// Whether the private key is backed by the Secure Enclave hardware.
    ///
    /// Returns `true` on physical devices where the Secure Enclave is used.
    /// Returns `false` in Simulator builds where a software P-256 key is
    /// used instead.
    public var isHardwareBacked: Bool {
        if case .secureEnclave = privateKey { return true }
        return false
    }

    /// Creates a new transformer with a fresh key.
    ///
    /// On physical devices the key is created in the Secure Enclave. In
    /// Simulator builds a software P-256 key is used instead.
    ///
    /// This is private because callers should use ``init(alias:)`` (which
    /// auto-persists the key) or ``init(persistedKeyData:)`` (which restores
    /// a previously saved key).  Using a bare `init()` by accident creates a
    /// new key every launch, making previously encrypted data unrecoverable.
    private init(createNewKey: Void = ()) throws {
        #if targetEnvironment(simulator)
        self.privateKey = .software(P256.KeyAgreement.PrivateKey())
        #else
        self.privateKey = .secureEnclave(try SecureEnclave.P256.KeyAgreement.PrivateKey())
        #endif
    }

    /// Creates a transformer from a previously persisted key.
    ///
    /// - Parameter persistedKeyData: The data obtained from ``persistedKeyData``
    ///   on a previously created transformer.
    /// - Throws: If the key data is invalid or was created with a key type
    ///   that is not available on this device.
    public init(persistedKeyData: Data) throws {
        guard let tag = persistedKeyData.first else {
            throw SecureEnclavePasswordTransformerError.invalidKeyData
        }

        let keyData = persistedKeyData.dropFirst()

        switch tag {
        case Self.seKeyTag:
            self.privateKey = .secureEnclave(
                try SecureEnclave.P256.KeyAgreement.PrivateKey(dataRepresentation: keyData)
            )
        case Self.softwareKeyTag:
            self.privateKey = .software(
                try P256.KeyAgreement.PrivateKey(rawRepresentation: keyData)
            )
        default:
            throw SecureEnclavePasswordTransformerError.invalidKeyData
        }
    }

    /// Creates a transformer whose key is automatically persisted to the
    /// Keychain under the given alias.
    ///
    /// On first use a new key is created and saved. On subsequent calls with
    /// the same alias the existing key is restored, so encrypted data
    /// survives app relaunches without any manual persistence.
    ///
    /// - Parameter alias: A Keychain account identifier for the stored key.
    ///   Different aliases produce independent keys.
    /// - Throws: ``SecureEnclavePasswordTransformerError/keychainError(_:)``
    ///   if Keychain access fails.
    public convenience init(
        alias: String = "wordpress-rs-password-transformer"
    ) throws {
        if let existing = try Self.loadFromKeychain(alias: alias) {
            try self.init(persistedKeyData: existing)
        } else {
            try self.init(createNewKey: ())
            try Self.saveToKeychain(alias: alias, data: persistedKeyData)
        }
    }

    /// Creates a transformer whose key is persisted to a file on disk.
    ///
    /// This is useful during development on macOS to avoid repeated Keychain
    /// password prompts caused by re-signing on each build. Place the key
    /// file next to your data store (e.g. alongside `accounts.json`).
    ///
    /// - Parameter keyFile: Path to the file where key data is stored.
    ///   Created automatically on first use.
    public convenience init(keyFile: URL) throws {
        let fm = FileManager.default
        if fm.fileExists(atPath: keyFile.path) {
            let data = try Data(contentsOf: keyFile)
            try self.init(persistedKeyData: data)
        } else {
            try self.init(createNewKey: ())
            try persistedKeyData.write(to: keyFile)
        }
    }

    /// An opaque representation of the private key that can be persisted
    /// across launches.
    ///
    /// On physical devices this is a Secure Enclave reference — it does not
    /// contain the actual private key and can only be used on the same
    /// device. In Simulator builds this contains the raw software key.
    public var persistedKeyData: Data {
        switch privateKey {
        case .secureEnclave(let key):
            var data = Data([Self.seKeyTag])
            data.append(key.dataRepresentation)
            return data
        case .software(let key):
            var data = Data([Self.softwareKeyTag])
            data.append(key.rawRepresentation)
            return data
        }
    }

    // MARK: - PasswordTransformer

    public func encrypt(password: String) throws -> String {
        let ephemeralKey = P256.KeyAgreement.PrivateKey()

        var salt = Data(count: Self.saltSize)
        let status = salt.withUnsafeMutableBytes {
            SecRandomCopyBytes(kSecRandomDefault, Self.saltSize, $0.baseAddress!)
        }
        guard status == errSecSuccess else {
            throw PasswordTransformerError.EncryptionFailed(
                reason: "Failed to generate cryptographically secure random bytes"
            )
        }

        let symmetricKey: SymmetricKey
        do {
            symmetricKey = try deriveKey(
                using: ephemeralKey,
                withPublicKey: privateKey.publicKey,
                salt: salt
            )
        } catch {
            throw PasswordTransformerError.EncryptionFailed(
                reason: "ECDH key agreement failed: \(error.localizedDescription)"
            )
        }

        let sealedBox: AES.GCM.SealedBox
        do {
            sealedBox = try AES.GCM.seal(
                Data(password.utf8), using: symmetricKey
            )
        } catch {
            throw PasswordTransformerError.EncryptionFailed(
                reason: "AES-GCM encryption failed: \(error.localizedDescription)"
            )
        }

        guard let sealedBoxCombined = sealedBox.combined else {
            throw PasswordTransformerError.EncryptionFailed(
                reason: "AES-GCM sealed box produced no combined output"
            )
        }

        var combined = Data()
        combined.append(salt)
        combined.append(ephemeralKey.publicKey.x963Representation)
        combined.append(contentsOf: sealedBoxCombined)

        return combined.base64EncodedString()
    }

    public func decrypt(password: String) throws -> String {
        guard let data = Data(base64Encoded: password) else {
            throw PasswordTransformerError.DecryptionFailed(
                reason: "Ciphertext is not valid base64"
            )
        }

        let minimumSize = Self.saltSize + Self.publicKeySize + 12 + 16
        guard data.count >= minimumSize else {
            throw PasswordTransformerError.DecryptionFailed(
                reason: "Ciphertext too short: expected at least \(minimumSize) bytes, got \(data.count)"
            )
        }

        let salt = data.prefix(Self.saltSize)
        let rest = data.dropFirst(Self.saltSize)

        let ephemeralPubKey: P256.KeyAgreement.PublicKey
        do {
            ephemeralPubKey = try P256.KeyAgreement.PublicKey(
                x963Representation: rest.prefix(Self.publicKeySize)
            )
        } catch {
            throw PasswordTransformerError.DecryptionFailed(
                reason: "Invalid ephemeral public key in ciphertext"
            )
        }

        let sharedSecret: SharedSecret
        do {
            sharedSecret = try privateKey.sharedSecretFromKeyAgreement(
                with: ephemeralPubKey
            )
        } catch {
            throw PasswordTransformerError.DecryptionFailed(
                reason: "ECDH key agreement failed: \(error.localizedDescription)"
            )
        }

        let symmetricKey = sharedSecret.hkdfDerivedSymmetricKey(
            using: SHA256.self,
            salt: salt,
            sharedInfo: Self.sharedInfo,
            outputByteCount: 32
        )

        let sealedBox: AES.GCM.SealedBox
        do {
            sealedBox = try AES.GCM.SealedBox(
                combined: rest.dropFirst(Self.publicKeySize)
            )
        } catch {
            throw PasswordTransformerError.DecryptionFailed(
                reason: "Invalid AES-GCM sealed box in ciphertext"
            )
        }

        let plaintext: Data
        do {
            plaintext = try AES.GCM.open(sealedBox, using: symmetricKey)
        } catch {
            throw PasswordTransformerError.DecryptionFailed(
                reason: "AES-GCM authentication failed — the data was likely encrypted with a different key"
            )
        }

        guard let result = String(bytes: plaintext, encoding: .utf8) else {
            throw PasswordTransformerError.DecryptionFailed(
                reason: "Decrypted data is not valid UTF-8"
            )
        }
        return result
    }

    // MARK: - Keychain Helpers

    private static let keychainService = "rs.wordpress.api.password-transformer"

    private static func loadFromKeychain(alias: String) throws -> Data? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: alias,
            kSecReturnData as String: true,
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        switch status {
        case errSecSuccess:
            return result as? Data
        case errSecItemNotFound:
            return nil
        default:
            throw SecureEnclavePasswordTransformerError.keychainError(status)
        }
    }

    private static func saveToKeychain(alias: String, data: Data) throws {
        let attributes: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: alias,
            kSecValueData as String: data,
        ]

        let status = SecItemAdd(attributes as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw SecureEnclavePasswordTransformerError.keychainError(status)
        }
    }

    // MARK: - Key Derivation

    private func deriveKey(
        using ephemeralKey: P256.KeyAgreement.PrivateKey,
        withPublicKey publicKey: P256.KeyAgreement.PublicKey,
        salt: Data
    ) throws -> SymmetricKey {
        let sharedSecret = try ephemeralKey.sharedSecretFromKeyAgreement(with: publicKey)
        return sharedSecret.hkdfDerivedSymmetricKey(
            using: SHA256.self,
            salt: salt,
            sharedInfo: Self.sharedInfo,
            outputByteCount: 32
        )
    }
}

// MARK: - Private Key Abstraction

/// Wraps either a Secure Enclave or software P-256 key agreement private key.
private enum EciesPrivateKey {
    case secureEnclave(SecureEnclave.P256.KeyAgreement.PrivateKey)
    case software(P256.KeyAgreement.PrivateKey)

    var publicKey: P256.KeyAgreement.PublicKey {
        switch self {
        case .secureEnclave(let key): key.publicKey
        case .software(let key): key.publicKey
        }
    }

    func sharedSecretFromKeyAgreement(
        with publicKey: P256.KeyAgreement.PublicKey
    ) throws -> SharedSecret {
        switch self {
        case .secureEnclave(let key):
            try key.sharedSecretFromKeyAgreement(with: publicKey)
        case .software(let key):
            try key.sharedSecretFromKeyAgreement(with: publicKey)
        }
    }
}

// MARK: - Errors

public enum SecureEnclavePasswordTransformerError: LocalizedError {
    case invalidKeyData
    case keychainError(OSStatus)

    public var errorDescription: String? {
        switch self {
        case .invalidKeyData:
            return "The persisted key data is invalid or corrupted."
        case .keychainError(let status):
            return "Keychain operation failed with status \(status)."
        }
    }
}
