#if canImport(CryptoKit)
import CryptoKit
import Foundation
import WordPressAPIInternal

/// A `PasswordTransformer` that encrypts passwords using the Secure Enclave
/// when available, falling back to a software key in Simulator builds.
///
/// The encryption key is generated once and persisted — either to the
/// Keychain via ``init(applicationName:)`` or to a file via
/// ``init(keyFile:)``. On physical devices the key never leaves the
/// Secure Enclave hardware. Check ``isHardwareBacked`` to determine
/// which mode is active.
///
/// ## Key Persistence
///
/// Use ``init(applicationName:)`` to auto-persist the key to the Keychain:
///
/// ```swift
/// // The key is created on first use and restored automatically on
/// // subsequent launches — no manual persistence needed.
/// let transformer = try SecureEnclavePasswordTransformer(applicationName: "my-app-key")
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
/// let transformer = try SecureEnclavePasswordTransformer(applicationName: "my-key")
///
/// let encrypted = try transformer.encrypt(password: "hunter2")
/// let decrypted = try transformer.decrypt(password: encrypted)
/// // decrypted == "hunter2"
/// ```
/// A storage backend for persisting encryption key data.
///
/// The default implementation (``SystemKeychainStorage``) uses the system
/// Keychain. Conforming types can provide alternative storage (e.g. in-memory)
/// for testing or environments where the Keychain is unavailable.
public protocol KeychainStorage {
    func load() throws -> Data?
    func save(data: Data) throws
}

/// The default ``KeychainStorage`` implementation that uses the system Keychain.
///
/// The `applicationName` is stored as the `kSecAttrService` value, which is
/// the most visible field in Keychain Access. This makes it easy to identify
/// which app owns a given Keychain item.
public struct SystemKeychainStorage: KeychainStorage {
    private let applicationName: String

    private static let accountName = "encryption-key"

    public init(applicationName: String) {
        self.applicationName = applicationName
    }

    public func load() throws -> Data? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: applicationName,
            kSecAttrAccount as String: Self.accountName,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
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

    public func save(data: Data) throws {
        let attributes: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: applicationName,
            kSecAttrAccount as String: Self.accountName,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
            kSecValueData as String: data
        ]

        let status = SecItemAdd(attributes as CFDictionary, nil)

        if status == errSecDuplicateItem {
            // SecItemAdd fails if the item already exists — this can happen
            // when two processes (e.g. the app and a share extension) race to
            // save the same key. Fall back to SecItemUpdate so the call is
            // idempotent rather than throwing an error.
            let query: [String: Any] = [
                kSecClass as String: kSecClassGenericPassword,
                kSecAttrService as String: applicationName,
                kSecAttrAccount as String: Self.accountName
            ]
            let update: [String: Any] = [
                kSecValueData as String: data,
                kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
            ]
            let updateStatus = SecItemUpdate(query as CFDictionary, update as CFDictionary)
            guard updateStatus == errSecSuccess else {
                throw SecureEnclavePasswordTransformerError.keychainError(updateStatus)
            }
        } else if status != errSecSuccess {
            throw SecureEnclavePasswordTransformerError.keychainError(status)
        }
    }
}

public final class SecureEnclavePasswordTransformer: PasswordTransformer {

    private let privateKey: PrivateKey

    /// The size of an uncompressed P-256 public key in X9.63 format.
    private static let publicKeySize = 65

    /// The size of the random HKDF salt generated per encryption.
    private static let saltSize = 32

    private static let sharedInfo = Data("wordpress-rs-password-encryption".utf8)

    private static let seKeyTag: UInt8 = 0x01
    private static let softwareKeyTag: UInt8 = 0x02

    /// Whether the private key is backed by the Secure Enclave hardware.
    ///
    /// Returns `true` on devices where the Secure Enclave is available and
    /// used. Returns `false` in Simulator builds or on machines without a
    /// Secure Enclave, where a software P-256 key is used instead.
    public var isHardwareBacked: Bool {
        if case .secureEnclave = privateKey { return true }
        return false
    }

    private init(privateKey: PrivateKey) {
        self.privateKey = privateKey
    }

    /// Creates a new key appropriate for the current platform.
    ///
    /// On iOS/tvOS/watchOS/visionOS devices the key is created in the Secure
    /// Enclave when available. On simulators a software P-256 key is used
    /// instead. On macOS the Secure Enclave is used when available (physical
    /// Macs with a T2 chip or Apple Silicon), otherwise a software key is used.
    private static func createKey() throws -> PrivateKey {
        #if targetEnvironment(simulator)
        .software(P256.KeyAgreement.PrivateKey())
        #else
        if SecureEnclave.isAvailable {
            .secureEnclave(try SecureEnclave.P256.KeyAgreement.PrivateKey())
        } else {
            .software(P256.KeyAgreement.PrivateKey())
        }
        #endif
    }

    /// Creates a transformer from a previously persisted key.
    ///
    /// - Parameter persistedKeyData: The data obtained from ``persistedKeyData``
    ///   on a previously created transformer.
    /// - Throws: If the key data is invalid or was created with a key type
    ///   that is not available on this device.
    public convenience init(persistedKeyData: Data) throws {
        self.init(privateKey: try Self.privateKey(from: persistedKeyData))
    }

    /// Creates a transformer whose key is automatically persisted via the
    /// given ``KeychainStorage``.
    ///
    /// On first use a new key is created and saved. On subsequent calls with
    /// the same application name the existing key is restored, so encrypted
    /// data survives app relaunches without any manual persistence.
    ///
    /// - Parameters:
    ///   - applicationName: An identifier for the stored key. Different names
    ///     produce independent keys. Used as `kSecAttrService` in the Keychain.
    ///   - keychainStorage: The storage backend used to persist the key.
    ///     Defaults to ``SystemKeychainStorage`` which uses the system Keychain.
    /// - Throws: ``SecureEnclavePasswordTransformerError/keychainError(_:)``
    ///   if storage access fails.
    public convenience init(
        applicationName: String,
        keychainStorage: KeychainStorage? = nil
    ) throws {
        let storage = keychainStorage ?? SystemKeychainStorage(applicationName: applicationName)
        if let existing = try storage.load() {
            try self.init(persistedKeyData: existing)
        } else {
            self.init(privateKey: try Self.createKey())
            try storage.save(data: persistedKeyData)
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
        let privateKey = try Self.resolveKeyFile(keyFile)
        self.init(privateKey: privateKey)
    }

    /// Loads the key from `keyFile` if it exists, or creates a new one and
    /// atomically persists it. If two processes race, the first writer wins
    /// and the other reads the winner's key — no data is lost.
    private static func resolveKeyFile(_ keyFile: URL) throws -> PrivateKey {
        // Fast path: file already exists.
        if let data = try? Data(contentsOf: keyFile) {
            return try privateKey(from: data)
        }

        // Create a new key and write it to a uniquely-named temp file.
        let newKey = try createKey()
        let tempTransformer = SecureEnclavePasswordTransformer(privateKey: newKey)
        let keyData = tempTransformer.persistedKeyData

        let tempFile = keyFile.appendingPathExtension("tmp-\(UUID().uuidString)")
        try keyData.write(to: tempFile)
        try FileManager.default.setAttributes(
            [.posixPermissions: 0o600],
            ofItemAtPath: tempFile.path
        )

        do {
            // Atomic move — succeeds only if no file exists at keyFile yet.
            try FileManager.default.moveItem(at: tempFile, to: keyFile)
            return newKey
        } catch {
            // Another process won the race. Clean up and use their key.
            try? FileManager.default.removeItem(at: tempFile)
            let winnerData = try Data(contentsOf: keyFile)
            return try privateKey(from: winnerData)
        }
    }

    /// Parses persisted key data into a `PrivateKey`.
    private static func privateKey(from persistedKeyData: Data) throws -> PrivateKey {
        guard let tag = persistedKeyData.first else {
            throw SecureEnclavePasswordTransformerError.invalidKeyData
        }

        let keyData = persistedKeyData.dropFirst()

        switch tag {
        case seKeyTag:
            return .secureEnclave(
                try SecureEnclave.P256.KeyAgreement.PrivateKey(dataRepresentation: keyData)
            )
        case softwareKeyTag:
            return .software(
                try P256.KeyAgreement.PrivateKey(rawRepresentation: keyData)
            )
        default:
            throw SecureEnclavePasswordTransformerError.invalidKeyData
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
private enum PrivateKey {
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

#endif
