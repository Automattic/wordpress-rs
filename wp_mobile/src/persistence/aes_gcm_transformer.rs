use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use hkdf::Hkdf;
use aes_gcm::aead::rand_core::RngCore;
use sha2::Sha256;
use zeroize::Zeroize;

use super::account_repository::{DecryptedPassword, EncryptedPassword, PasswordTransformer, PasswordTransformerError};

const SALT_SIZE: usize = 32;

/// AES-256-GCM implementation of `PasswordTransformer`.
///
/// Each encryption generates a random 32-byte HKDF salt, which is included
/// in the output so that decryption can re-derive the same key. The encrypted
/// output is base64-encoded as `salt:nonce:ciphertext`.
///
/// # Shared secret management
///
/// The `shared_secret` passed to [`AesGcmPasswordTransformer::new`] must come
/// from a secure, platform-managed store:
///
/// - **iOS**: Use the Keychain (`kSecClassGenericPassword`)
/// - **Android**: Use the Android Keystore
///
/// Never hardcode the shared secret in the application binary or derive it
/// from predictable values. If the secret is compromised, all encrypted
/// passwords can be decrypted.
///
/// # Example
///
/// ```rust
/// use wp_mobile::persistence::{AesGcmPasswordTransformer, PasswordTransformer, DecryptedPassword};
///
/// // Create a transformer with a shared secret from a secure store
/// let transformer = AesGcmPasswordTransformer::new("secret-from-keychain".to_string());
///
/// // Encrypt a password
/// let encrypted = transformer.encrypt(DecryptedPassword("hunter2".to_string())).unwrap();
///
/// // Decrypt it back
/// let decrypted = transformer.decrypt(encrypted).unwrap();
/// assert_eq!(decrypted.0, "hunter2");
/// ```
#[derive(uniffi::Object)]
pub struct AesGcmPasswordTransformer {
    master_key: [u8; 32],
}

impl Drop for AesGcmPasswordTransformer {
    fn drop(&mut self) {
        self.master_key.zeroize();
    }
}

#[uniffi::export]
impl AesGcmPasswordTransformer {
    #[uniffi::constructor]
    pub fn new(mut shared_secret: String) -> Self {
        // Derive a master key from the shared secret, then zeroize the secret.
        let hkdf = Hkdf::<Sha256>::new(None, shared_secret.as_bytes());
        let mut master_key = [0u8; 32];
        hkdf.expand(b"wordpress-rs-master-key", &mut master_key)
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        shared_secret.zeroize();

        Self { master_key }
    }
}

impl AesGcmPasswordTransformer {
    fn derive_key(&self, salt: &[u8]) -> Key<Aes256Gcm> {
        let hkdf = Hkdf::<Sha256>::new(Some(salt), &self.master_key);
        let mut key_bytes = [0u8; 32];
        hkdf.expand(b"wordpress-rs-password-encryption", &mut key_bytes)
            .expect("32 bytes is a valid HKDF-SHA256 output length");

        let key = Key::<Aes256Gcm>::from(key_bytes);
        key_bytes.zeroize();
        key
    }
}

#[uniffi::export]
impl PasswordTransformer for AesGcmPasswordTransformer {
    fn encrypt(&self, password: DecryptedPassword) -> Result<EncryptedPassword, PasswordTransformerError> {
        let mut salt = [0u8; SALT_SIZE];
        OsRng.fill_bytes(&mut salt);

        let mut key = self.derive_key(&salt);
        let cipher = Aes256Gcm::new(&key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let result = cipher
            .encrypt(&nonce, password.0.as_bytes())
            .map_err(|e| PasswordTransformerError::EncryptionFailed {
                reason: e.to_string(),
            });
        key.zeroize();
        let ciphertext = result?;

        let encoded = format!(
            "{}:{}:{}",
            BASE64.encode(salt),
            BASE64.encode(nonce),
            BASE64.encode(ciphertext)
        );
        Ok(EncryptedPassword(encoded))
    }

    fn decrypt(&self, password: EncryptedPassword) -> Result<DecryptedPassword, PasswordTransformerError> {
        let parse_err = |msg: &str| PasswordTransformerError::DecryptionFailed {
            reason: msg.to_string(),
        };

        let mut parts = password.0.splitn(3, ':');
        let salt_b64 = parts.next().ok_or_else(|| parse_err("invalid encrypted password format"))?;
        let nonce_b64 = parts.next().ok_or_else(|| parse_err("invalid encrypted password format"))?;
        let ciphertext_b64 = parts.next().ok_or_else(|| parse_err("invalid encrypted password format"))?;

        let salt = BASE64.decode(salt_b64).map_err(|e| PasswordTransformerError::DecryptionFailed {
            reason: e.to_string(),
        })?;
        let nonce_bytes = BASE64.decode(nonce_b64).map_err(|e| PasswordTransformerError::DecryptionFailed {
            reason: e.to_string(),
        })?;
        if nonce_bytes.len() != 12 {
            return Err(PasswordTransformerError::DecryptionFailed {
                reason: format!("invalid nonce length: expected 12, got {}", nonce_bytes.len()),
            });
        }
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = BASE64.decode(ciphertext_b64).map_err(|e| PasswordTransformerError::DecryptionFailed {
            reason: e.to_string(),
        })?;

        let mut key = self.derive_key(&salt);
        let cipher = Aes256Gcm::new(&key);

        let result = cipher
            .decrypt(&nonce, ciphertext.as_ref())
            .map_err(|e| PasswordTransformerError::DecryptionFailed {
                reason: e.to_string(),
            });
        key.zeroize();
        let plaintext = result?;

        String::from_utf8(plaintext)
            .map(DecryptedPassword)
            .map_err(|e| PasswordTransformerError::DecryptionFailed {
                reason: e.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transformer() -> AesGcmPasswordTransformer {
        AesGcmPasswordTransformer::new("my-shared-secret".to_string())
    }

    #[test]
    fn test_round_trip() {
        let t = transformer();
        let original = DecryptedPassword("hunter2".to_string());

        let encrypted = t.encrypt(original.clone()).unwrap();
        let decrypted = t.decrypt(encrypted).unwrap();

        assert_eq!(decrypted.0, original.0);
    }

    #[test]
    fn test_encrypted_differs_from_plaintext() {
        let t = transformer();
        let encrypted = t.encrypt(DecryptedPassword("hunter2".to_string())).unwrap();

        assert_ne!(encrypted.0, "hunter2");
    }

    #[test]
    fn test_different_nonces_produce_different_ciphertext() {
        let t = transformer();
        let password = DecryptedPassword("hunter2".to_string());

        let enc1 = t.encrypt(password.clone()).unwrap();
        let enc2 = t.encrypt(password).unwrap();

        assert_ne!(enc1.0, enc2.0);
    }

    #[test]
    fn test_wrong_key_fails_to_decrypt() {
        let t1 = AesGcmPasswordTransformer::new("key-one".to_string());
        let t2 = AesGcmPasswordTransformer::new("key-two".to_string());

        let encrypted = t1.encrypt(DecryptedPassword("secret".to_string())).unwrap();

        assert!(t2.decrypt(encrypted).is_err());
    }

    #[test]
    fn test_empty_password_round_trip() {
        let t = transformer();
        let original = DecryptedPassword(String::new());

        let encrypted = t.encrypt(original.clone()).unwrap();
        let decrypted = t.decrypt(encrypted).unwrap();

        assert_eq!(decrypted.0, original.0);
    }

    #[test]
    fn test_long_password_round_trip() {
        let t = transformer();
        let long = "a".repeat(10_000);
        let original = DecryptedPassword(long.clone());

        let encrypted = t.encrypt(original).unwrap();
        let decrypted = t.decrypt(encrypted).unwrap();

        assert_eq!(decrypted.0, long);
    }

    #[test]
    fn test_unicode_password_round_trip() {
        let t = transformer();
        let original = DecryptedPassword("p@$$w0rd-\u{1F512}-\u{00E9}\u{00F1}".to_string());

        let encrypted = t.encrypt(original.clone()).unwrap();
        let decrypted = t.decrypt(encrypted).unwrap();

        assert_eq!(decrypted.0, original.0);
    }

    #[test]
    fn test_debug_does_not_leak_plaintext() {
        let password = DecryptedPassword("super-secret".to_string());
        let debug_output = format!("{:?}", password);
        assert!(!debug_output.contains("super-secret"));
        assert!(debug_output.contains("***"));
    }
}
